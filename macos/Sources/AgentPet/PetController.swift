import AppKit

/// Owns the surface, the two clocks that drive it, and where it sits.
///
/// Discovery runs on its own slower clock; the display ticks every second purely
/// to advance the age counters. Polling rather than watching the filesystem is
/// deliberate: a force-killed session changes no file on disk, so only an active
/// check notices it has gone.
///
/// The surface is a creature with the rows in a bubble above it. The window's
/// frame is both together, and the creature is the fixed point: the bubble hangs
/// off whichever side of it fits, and every move — a drag, a row appearing, a
/// display going away — is expressed as *where the creature is and which way the
/// bubble runs*, then turned into a frame by `PetGeometry`. Keeping one direction
/// of derivation is what stops the two halves drifting apart.
final class PetController: NSObject {
    private static let discoveryInterval: TimeInterval = 2
    private static let displayInterval: TimeInterval = 1

    /// Where the remembered position lives. Not the pet's config file: window
    /// geometry is a macOS surface concern the observation core has no business
    /// knowing, and that file is hand-edited — an app that writes to it can
    /// clobber what the user typed.
    ///
    /// `edgeY` and `anchor` are story 002's keys and still mean what they meant:
    /// the frame's top under a top anchor, its bottom under a bottom one. Only
    /// `legacyFrameX` changed meaning — it was the frame's left edge, which is now
    /// the creature's only when the bubble happens to run right — so it is
    /// replaced by `creatureX` rather than reinterpreted in place, read once to
    /// migrate and then left alone forever.
    private enum Key {
        static let creatureX = "petCreatureX"
        static let side = "petBubbleSide"
        static let edgeY = "petPositionEdgeY"
        static let anchor = "petPositionAnchor"
        static let legacyFrameX = "petPositionX"
    }

    private let panel: PetPanel
    private let content = NSView()
    private let bubble = BubbleView(frame: .zero)
    private let creatureView = CreatureView()
    private let container = NSStackView()
    private let statusLabel = Style.label("", font: Style.detailFont, color: Style.secondary)
    private var rows: [String: SessionRowView] = [:]
    private var discoveryTimer: Timer?
    private var displayTimer: Timer?
    private var menuBar: MenuBarController?
    private var isShowing = true

    /// Which way the bubble currently runs from the creature. Held here rather
    /// than read back off the frame, because a frame alone cannot say which end
    /// the creature is at.
    private var side = defaultSide
    /// Where the creature was when the current drag began.
    private var dragOrigin: CGRect?
    /// Held so the monitors outlive this call and can be torn down with the pet.
    private var pointerMonitors: [Any?] = []

    override init() {
        panel = PetPanel(contentRect: NSRect(
            x: 0,
            y: 0,
            width: Style.width,
            height: CreatureView.size.height + 60
        ))
        super.init()

        container.orientation = .vertical
        container.alignment = .leading
        container.spacing = 8
        container.translatesAutoresizingMaskIntoConstraints = false
        bubble.addSubview(container)
        NSLayoutConstraint.activate([
            container.leadingAnchor.constraint(equalTo: bubble.leadingAnchor, constant: Style.padding),
            container.trailingAnchor.constraint(equalTo: bubble.trailingAnchor, constant: -Style.padding),
            container.topAnchor.constraint(equalTo: bubble.topAnchor, constant: Style.padding),
        ])

        content.addSubview(creatureView)
        content.addSubview(bubble)
        panel.contentView = content

        creatureView.onDragBegin = { [weak self] in
            guard let self else { return }
            dragOrigin = creatureRect(in: panel.frame, side: side, size: CreatureView.size)
        }
        creatureView.onDrag = { [weak self] delta in self?.drag(by: delta) }
        creatureView.onDragEnd = { [weak self] in
            self?.dragOrigin = nil
            self?.savePosition()
        }
    }

    func start() {
        render(Core.poll())
        applyPlacement()
        // `orderFrontRegardless` shows the panel without activating the app, so
        // nothing the user is typing into is interrupted.
        panel.orderFrontRegardless()

        menuBar = MenuBarController(
            onToggle: { [weak self] in self?.toggleVisibility() },
            onQuit: { NSApp.terminate(nil) }
        )
        menuBar?.showPetIsVisible(true)

        let discovery = Timer(timeInterval: Self.discoveryInterval, repeats: true) { [weak self] _ in
            self?.render(Core.poll())
        }
        // Keep it ticking while menus or resizes are running the loop in a
        // tracking mode, so the pet never silently freezes.
        RunLoop.main.add(discovery, forMode: .common)
        discoveryTimer = discovery
        startDisplayClock()

        watchPointerForClickThrough()

        // A display disconnected or resized can strand the pet where no screen
        // reaches, or leave the bubble hanging off the edge of the one that is
        // left. Same rules as launch, so the two cannot drift apart.
        NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.revalidatePlacement()
        }
    }

    // MARK: - Showing and hiding

    /// Hiding stops the display clock but not discovery.
    ///
    /// The age labels are the only thing the one-second clock touches, and while
    /// hidden nobody can read them. Discovery keeps running: it costs about 1.5%
    /// of a core, and pausing it would leave a gap in what the pet knows for the
    /// sake of a saving nobody would notice.
    private func toggleVisibility() {
        isShowing.toggle()
        if isShowing {
            panel.orderFrontRegardless()
            rows.values.forEach { $0.tick() }
            startDisplayClock()
        } else {
            panel.orderOut(nil)
            stopDisplayClock()
        }
        menuBar?.showPetIsVisible(isShowing)
    }

    private func startDisplayClock() {
        guard displayTimer == nil else { return }
        let display = Timer(timeInterval: Self.displayInterval, repeats: true) { [weak self] _ in
            self?.rows.values.forEach { $0.tick() }
        }
        RunLoop.main.add(display, forMode: .common)
        displayTimer = display
    }

    private func stopDisplayClock() {
        displayTimer?.invalidate()
        displayTimer = nil
    }

    // MARK: - What a click lands on

    /// The parts of the surface that are actually drawn.
    ///
    /// The bubble's solid part stops above the tail band, which is transparent
    /// apart from the tail itself; the creature's whole box counts, which is more
    /// forgiving to grab than its outline and costs only the corners of a
    /// 64-point square.
    private var solidRegions: [NSRect] {
        [
            creatureView.frame,
            bubble.frame.divided(atDistance: BubbleView.tail, from: .minYEdge).remainder,
        ]
    }

    /// Let clicks through the empty band beside the creature.
    ///
    /// A 300-point bubble with a 64-point creature under one corner leaves a wide
    /// transparent gap, and the pet floats over the user's actual work: swallowing
    /// clicks where they can see the window underneath is the trap story 001 hit,
    /// in a new place. What is *drawn* still catches its own clicks — the bubble
    /// consumes them and does nothing, the creature drags.
    ///
    /// `ignoresMouseEvents` is the only thing that actually does this. Declining
    /// the point in `hitTest` looks like it should and does not: the window still
    /// consumes the event, it simply has no view to hand it to. Measured, not
    /// assumed — a synthetic click in the gap left the window beneath untouched
    /// with a `hitTest` override in place.
    ///
    /// Two monitors because each is blind to what the other sees: the global one
    /// misses events delivered to this app, the local one misses everything while
    /// the panel is ignoring events. Together they cover the pointer wherever it is.
    private func watchPointerForClickThrough() {
        panel.acceptsMouseMovedEvents = true
        pointerMonitors = [
            NSEvent.addGlobalMonitorForEvents(matching: [.mouseMoved]) { [weak self] _ in
                self?.updateClickThrough()
            },
            NSEvent.addLocalMonitorForEvents(matching: [.mouseMoved]) { [weak self] event in
                self?.updateClickThrough()
                return event
            },
        ]
        updateClickThrough()
    }

    private func updateClickThrough() {
        let pointer = panel.convertPoint(fromScreen: NSEvent.mouseLocation)
        let overSomethingDrawn = solidRegions.contains { $0.contains(pointer) }
        guard panel.ignoresMouseEvents == overSomethingDrawn else { return }
        panel.ignoresMouseEvents = !overSomethingDrawn
    }

    // MARK: - Placement

    private var screens: [NSRect] { NSScreen.screens.map(\.visibleFrame) }

    /// The display holding the menu bar.
    ///
    /// Not `NSScreen.main`, which is the screen containing the key window: this
    /// panel is never key and the app never activates, so `main` reports whatever
    /// some other app happens to have focused — or nothing at all. That is the
    /// wrong thing to fall back to precisely when it matters, deciding where a
    /// pet stranded by an unplugged display should reappear.
    private func primaryVisibleFrame() -> NSRect {
        NSScreen.screens.first?.visibleFrame ?? .zero
    }

    private func screen(nearest rect: NSRect) -> NSRect {
        nearestVisible(to: rect, among: screens, fallback: primaryVisibleFrame())
    }

    /// The bubble's height, tail included, at the surface's current frame.
    private var bubbleHeight: CGFloat { panel.frame.height - CreatureView.size.height }

    private var currentCreature: NSRect {
        creatureRect(in: panel.frame, side: side, size: CreatureView.size)
    }

    private func applyPlacement() {
        let screens = screens
        let primary = primaryVisibleFrame()
        let migrated = loadPosition()
        let target = placement(
            remembered: migrated?.position,
            creatureSize: CreatureView.size,
            bubbleWidth: Style.width,
            bubbleHeight: bubbleHeight,
            visibleFrames: screens,
            primary: primary
        )
        side = target.side
        setFrame(target.frame)
        // The one write that is not a drag: story 002's position, restated under
        // the new keys so the old one is never consulted again. What is written is
        // what was migrated, with the side that was actually taken — not a fresh
        // reading of the frame, which would re-derive the anchored edge from a
        // surface whose height has changed since story 002 wrote it and could quietly
        // flip a top-anchored pet to the bottom.
        if let migrated, migrated.fromLegacyKeys {
            var position = migrated.position
            position.side = target.side
            write(position)
        }
    }

    /// A display change re-picks the bubble's side and, only where the creature
    /// itself has been stranded, moves the pet.
    ///
    /// The creature is what is tested, not the frame: a bubble hanging off an edge
    /// is something the side rule fixes on the spot, and throwing a perfectly
    /// reachable creature back to the top-right over it would move a pet the user
    /// can still see. AppKit has usually already relocated the window onto a
    /// remaining screen by the time this runs, which is why the creature is so
    /// often exactly where it was and only the bubble moves.
    private func revalidatePlacement() {
        let creature = currentCreature
        guard isRestorable(creature, on: screens) else {
            side = defaultSide
            setFrame(defaultFrame(size: panel.frame.size, in: primaryVisibleFrame()))
            return
        }
        let picked = bubbleSide(
            creature: creature,
            current: side,
            bubbleWidth: Style.width,
            in: screen(nearest: creature)
        )
        guard picked != side else { return }
        side = picked
        setFrame(petFrame(
            creature: creature,
            side: picked,
            bubbleWidth: Style.width,
            bubbleHeight: bubbleHeight
        ))
    }

    /// The creature follows the pointer exactly; the bubble re-picks its side as
    /// it goes, so it flips live rather than snapping into place on release.
    private func drag(by delta: CGSize) {
        guard let dragOrigin else { return }
        let creature = dragOrigin.offsetBy(dx: delta.width, dy: delta.height)
        side = bubbleSide(
            creature: creature,
            current: side,
            bubbleWidth: Style.width,
            in: screen(nearest: creature)
        )
        setFrame(petFrame(
            creature: creature,
            side: side,
            bubbleWidth: Style.width,
            bubbleHeight: bubbleHeight
        ))
    }

    /// Every frame change goes through here, so the creature and the bubble are
    /// never laid out against a frame the window no longer has.
    private func setFrame(_ frame: NSRect) {
        panel.setFrame(frame, display: true)
        layoutSurface()
    }

    private func layoutSurface() {
        let size = panel.frame.size
        let creature = CGRect(
            x: side == .left ? size.width - CreatureView.size.width : 0,
            y: 0,
            width: CreatureView.size.width,
            height: CreatureView.size.height
        )
        creatureView.frame = creature
        bubble.frame = CGRect(
            x: 0,
            y: CreatureView.size.height,
            width: size.width,
            height: size.height - CreatureView.size.height
        )
        bubble.pointTail(at: creature.midX)
    }

    /// Where the pet is now, reduced to what gets remembered.
    private func currentPosition() -> StoredPosition {
        stored(
            panel.frame,
            side: side,
            creatureSize: CreatureView.size,
            in: screen(nearest: panel.frame)
        )
    }

    private func savePosition() {
        write(currentPosition())
    }

    private func write(_ position: StoredPosition) {
        let defaults = UserDefaults.standard
        defaults.set(Double(position.x), forKey: Key.creatureX)
        defaults.set(Double(position.edgeY), forKey: Key.edgeY)
        defaults.set(position.anchor.rawValue, forKey: Key.anchor)
        defaults.set(position.side.rawValue, forKey: Key.side)
    }

    /// Nothing remembered is the normal first run, not an error.
    private func loadPosition() -> (position: StoredPosition, fromLegacyKeys: Bool)? {
        let defaults = UserDefaults.standard
        guard defaults.object(forKey: Key.edgeY) != nil,
              let rawAnchor = defaults.string(forKey: Key.anchor),
              let anchor = VerticalAnchor(rawValue: rawAnchor)
        else { return nil }
        let edgeY = CGFloat(defaults.double(forKey: Key.edgeY))

        if defaults.object(forKey: Key.creatureX) != nil,
           let rawSide = defaults.string(forKey: Key.side),
           let side = BubbleSide(rawValue: rawSide) {
            return (
                StoredPosition(
                    x: CGFloat(defaults.double(forKey: Key.creatureX)),
                    edgeY: edgeY,
                    anchor: anchor,
                    side: side
                ),
                false
            )
        }

        // Story 002 remembered the bubble alone. Rebuild the frame it described and
        // put the creature under its right corner, so the bubble lands where the
        // user last saw it; `placement` re-picks the side if it no longer fits.
        guard defaults.object(forKey: Key.legacyFrameX) != nil else { return nil }
        let height = CreatureView.size.height + bubbleHeight
        let legacy = CGRect(
            x: CGFloat(defaults.double(forKey: Key.legacyFrameX)),
            y: anchor == .top ? edgeY - height : edgeY,
            width: Style.width,
            height: height
        )
        let derived = derivedCreature(fromLegacyFrame: legacy, creatureWidth: CreatureView.size.width)
        return (
            StoredPosition(x: derived.x, edgeY: edgeY, anchor: anchor, side: derived.side),
            true
        )
    }

    // MARK: - Drawing

    /// Draw one poll.
    ///
    /// Three situations have nothing normal to show and each says something
    /// different, so none can be mistaken for another: discovery failing outright,
    /// discovery succeeding with nothing running, and a single session whose state
    /// could not be read (handled on the row itself).
    private func render(_ poll: Poll) {
        guard poll.ok else {
            if let detail = poll.error {
                // Useful to whoever has to fix it, without crowding the surface.
                FileHandle.standardError.write(Data("agent-pet: discovery failed: \(detail)\n".utf8))
            }
            // Unknown, not errored: the pet cannot see, and no agent has failed.
            creatureView.apply(.unknown)
            showMessage("Sessions unreadable")
            return
        }
        guard !poll.sessions.isEmpty else {
            creatureView.apply(.asleep)
            showMessage("No agents running")
            return
        }

        statusLabel.removeFromSuperview()

        let live = Set(poll.sessions.map(\.sessionKey))
        for (key, view) in rows where !live.contains(key) {
            container.removeView(view)
            rows.removeValue(forKey: key)
        }
        for (index, session) in poll.sessions.enumerated() {
            if let existing = rows[session.sessionKey] {
                existing.apply(session)
                if container.arrangedSubviews.firstIndex(of: existing) != index {
                    container.removeView(existing)
                    container.insertArrangedSubview(existing, at: index)
                }
            } else {
                let row = SessionRowView(session: session)
                rows[session.sessionKey] = row
                container.insertArrangedSubview(row, at: min(index, container.arrangedSubviews.count))
                row.widthAnchor.constraint(equalTo: container.widthAnchor).isActive = true
            }
        }

        // The creature is a function of this poll and nothing else — no memory of
        // the last one, so a session flapping between states flaps the creature
        // with it, exactly as its row's dot already does.
        let states = poll.sessions.map(\.state)
        creatureView.apply(aggregate(states).map(Expression.init) ?? .asleep)
        resize()
    }

    /// The empty and error surfaces both replace the list entirely.
    private func showMessage(_ text: String) {
        rows.values.forEach { container.removeView($0) }
        rows.removeAll()
        statusLabel.stringValue = text
        if statusLabel.superview == nil {
            container.addArrangedSubview(statusLabel)
        }
        resize()
    }

    /// Grow away from whichever edge the pet is sitting against, so rows coming
    /// and going never walk it off the screen and never move the edge the user
    /// lined it up with. Under a top anchor that carries the creature down with
    /// the bubble's bottom; under a bottom anchor the creature stays exactly where
    /// it is and the bubble grows upward over it.
    private func resize() {
        container.layoutSubtreeIfNeeded()
        let bubbleHeight = container.fittingSize.height + Style.padding * 2 + BubbleView.tail
        let size = NSSize(width: Style.width, height: CreatureView.size.height + bubbleHeight)
        let edge = anchor(for: panel.frame, in: screen(nearest: panel.frame))
        setFrame(resized(panel.frame, to: size, anchoredAt: edge))
    }
}
