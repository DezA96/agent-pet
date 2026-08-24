import AppKit

/// Owns the surface and the two clocks that drive it.
///
/// Discovery runs on its own slower clock; the display ticks every second purely
/// to advance the age counters. Polling rather than watching the filesystem is
/// deliberate: a force-killed session changes no file on disk, so only an active
/// check notices it has gone.
final class PetController {
    private static let discoveryInterval: TimeInterval = 2
    private static let displayInterval: TimeInterval = 1

    private let panel: PetPanel
    private let container = NSStackView()
    private let statusLabel = Style.label("", font: Style.detailFont, color: Style.secondary)
    private var rows: [String: SessionRowView] = [:]
    private var discoveryTimer: Timer?
    private var displayTimer: Timer?

    init() {
        panel = PetPanel(contentRect: NSRect(x: 0, y: 0, width: Style.width, height: 60))

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
        panel.contentView = backdrop
    }

    func start() {
        render(Core.poll())
        // `orderFrontRegardless` shows the panel without activating the app, so
        // nothing the user is typing into is interrupted.
        panel.orderFrontRegardless()
        position()

        let discovery = Timer(timeInterval: Self.discoveryInterval, repeats: true) { [weak self] _ in
            self?.render(Core.poll())
        }
        let display = Timer(timeInterval: Self.displayInterval, repeats: true) { [weak self] _ in
            self?.rows.values.forEach { $0.refreshAge() }
        }
        // Keep both ticking while menus or resizes are running the loop in a
        // tracking mode, so the pet never silently freezes.
        RunLoop.main.add(discovery, forMode: .common)
        RunLoop.main.add(display, forMode: .common)
        discoveryTimer = discovery
        displayTimer = display
    }

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

    private func resize() {
        container.layoutSubtreeIfNeeded()
        let height = container.fittingSize.height + Style.padding * 2
        var frame = panel.frame
        // Grow downward from the top edge so the pet stays put as rows come and go.
        frame.origin.y += frame.height - height
        frame.size = NSSize(width: Style.width, height: height)
        panel.setFrame(frame, display: true)
    }

    /// Top-right of the screen holding the menu bar, clear of it.
    private func position() {
        guard let screen = NSScreen.main else { return }
        let visible = screen.visibleFrame
        let frame = panel.frame
        panel.setFrameOrigin(NSPoint(
            x: visible.maxX - frame.width - 16,
            y: visible.maxY - frame.height - 16
        ))
    }
}
