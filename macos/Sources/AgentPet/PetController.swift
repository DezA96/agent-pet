import AppKit

/// Owns the surface, the two clocks that drive it, and where it sits.
///
/// Discovery runs on its own slower clock; the display ticks every second purely
/// to advance the age counters. Polling rather than watching the filesystem is
/// deliberate: a force-killed session changes no file on disk, so only an active
/// check notices it has gone.
final class PetController: NSObject, NSWindowDelegate {
    private static let discoveryInterval: TimeInterval = 2
    private static let displayInterval: TimeInterval = 1

    /// Where the remembered position lives. Not the pet's config file: window
    /// geometry is a macOS surface concern the observation core has no business
    /// knowing, and that file is hand-edited — an app that writes to it can
    /// clobber what the user typed.
    private enum Key {
        static let x = "petPositionX"
        static let edgeY = "petPositionEdgeY"
        static let anchor = "petPositionAnchor"
    }

    private let panel: PetPanel
    private let container = NSStackView()
    private let statusLabel = Style.label("", font: Style.detailFont, color: Style.secondary)
    private var rows: [String: SessionRowView] = [:]
    private var discoveryTimer: Timer?
    private var displayTimer: Timer?
    private var menuBar: MenuBarController?
    private var isShowing = true
    /// Set while the controller is moving the panel itself, so its own placement
    /// is not mistaken for the user dragging.
    private var isPlacing = false

    override init() {
        panel = PetPanel(contentRect: NSRect(x: 0, y: 0, width: Style.width, height: 60))
        super.init()

        let backdrop = NSVisualEffectView()
        backdrop.material = .hudWindow
        backdrop.blendingMode = .behindWindow
        backdrop.state = .active
        backdrop.wantsLayer = true
        backdrop.layer?.cornerRadius = 10
        backdrop.layer?.masksToBounds = true

        container.orientation = .vertical
        container.alignment = .leading
        container.spacing = 8
        container.translatesAutoresizingMaskIntoConstraints = false
        backdrop.addSubview(container)
        NSLayoutConstraint.activate([
            container.leadingAnchor.constraint(equalTo: backdrop.leadingAnchor, constant: Style.padding),
            container.trailingAnchor.constraint(equalTo: backdrop.trailingAnchor, constant: -Style.padding),
            container.topAnchor.constraint(equalTo: backdrop.topAnchor, constant: Style.padding),
            container.bottomAnchor.constraint(equalTo: backdrop.bottomAnchor, constant: -Style.padding),
        ])

        // Above the rows, so every point of the pet is a drag handle.
        let drag = DragOverlayView()
        drag.translatesAutoresizingMaskIntoConstraints = false
        backdrop.addSubview(drag)
        NSLayoutConstraint.activate([
            drag.leadingAnchor.constraint(equalTo: backdrop.leadingAnchor),
            drag.trailingAnchor.constraint(equalTo: backdrop.trailingAnchor),
            drag.topAnchor.constraint(equalTo: backdrop.topAnchor),
            drag.bottomAnchor.constraint(equalTo: backdrop.bottomAnchor),
        ])

        panel.contentView = backdrop
        panel.delegate = self
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

        // A display disconnected or resized can strand the pet where no screen
        // reaches. Same rule as launch, so the two cannot drift apart.
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

    // MARK: - Placement

    /// The screen showing most of the pet, falling back to the main one.
    private func visibleFrame(nearest frame: NSRect) -> NSRect {
        let best = NSScreen.screens.max { a, b in
            overlapArea(frame, a.visibleFrame) < overlapArea(frame, b.visibleFrame)
        }
        if let best, overlapArea(frame, best.visibleFrame) > 0 { return best.visibleFrame }
        return primaryVisibleFrame()
    }

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

    private func overlapArea(_ a: NSRect, _ b: NSRect) -> CGFloat {
        let i = a.intersection(b)
        return i.isNull ? 0 : i.width * i.height
    }

    private func applyPlacement() {
        let screens = NSScreen.screens.map(\.visibleFrame)
        let primary = primaryVisibleFrame()
        let target = placement(
            remembered: loadPosition(),
            size: panel.frame.size,
            visibleFrames: screens,
            primary: primary
        )
        setFrameWithoutRemembering(target)
    }

    /// Only act when the pet has actually been stranded, so a display change
    /// never moves a pet the user can still see.
    private func revalidatePlacement() {
        let screens = NSScreen.screens.map(\.visibleFrame)
        guard !isRestorable(panel.frame, on: screens) else { return }
        let primary = primaryVisibleFrame()
        setFrameWithoutRemembering(defaultFrame(size: panel.frame.size, in: primary))
    }

    private func setFrameWithoutRemembering(_ frame: NSRect) {
        isPlacing = true
        panel.setFrame(frame, display: true)
        isPlacing = false
    }

    func windowDidMove(_ notification: Notification) {
        guard !isPlacing else { return }
        savePosition()
    }

    private func savePosition() {
        let position = stored(panel.frame, in: visibleFrame(nearest: panel.frame))
        let defaults = UserDefaults.standard
        defaults.set(Double(position.x), forKey: Key.x)
        defaults.set(Double(position.edgeY), forKey: Key.edgeY)
        defaults.set(position.anchor.rawValue, forKey: Key.anchor)
    }

    /// Nothing remembered is the normal first run, not an error.
    private func loadPosition() -> StoredPosition? {
        let defaults = UserDefaults.standard
        guard defaults.object(forKey: Key.x) != nil,
              defaults.object(forKey: Key.edgeY) != nil,
              let raw = defaults.string(forKey: Key.anchor),
              let anchor = VerticalAnchor(rawValue: raw)
        else { return nil }
        return StoredPosition(
            x: CGFloat(defaults.double(forKey: Key.x)),
            edgeY: CGFloat(defaults.double(forKey: Key.edgeY)),
            anchor: anchor
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
            showMessage("Sessions unreadable")
            return
        }
        guard !poll.sessions.isEmpty else {
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
    /// lined it up with.
    private func resize() {
        container.layoutSubtreeIfNeeded()
        let height = container.fittingSize.height + Style.padding * 2
        let size = NSSize(width: Style.width, height: height)
        let edge = anchor(for: panel.frame, in: visibleFrame(nearest: panel.frame))
        setFrameWithoutRemembering(resized(panel.frame, to: size, anchoredAt: edge))
    }
}
