import AppKit

/// One live session.
///
/// Layout is: project name, then the state or the activity line, then a count-up
/// of seconds since the status was observed. The age is always on screen so
/// staleness is visible directly rather than inferred from a tuned threshold.
final class SessionRowView: NSView {
    private let nameLabel = Style.label("", font: Style.rowFont, color: Style.primary)
    private let detailLabel = Style.label("", font: Style.detailFont, color: Style.secondary)
    private let ageLabel = Style.label("", font: Style.detailFont, color: Style.secondary)

    private(set) var session: AgentSession

    init(session: AgentSession) {
        self.session = session
        super.init(frame: .zero)

        ageLabel.alignment = .right
        ageLabel.setContentHuggingPriority(.required, for: .horizontal)
        ageLabel.setContentCompressionResistancePriority(.required, for: .horizontal)
        nameLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        detailLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        let top = NSStackView(views: [nameLabel, ageLabel])
        top.orientation = .horizontal
        top.distribution = .fill
        top.spacing = 8

        let stack = NSStackView(views: [top, detailLabel])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 1
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor),
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
            top.widthAnchor.constraint(equalTo: stack.widthAnchor),
        ])

        apply(session)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func apply(_ session: AgentSession) {
        self.session = session
        nameLabel.stringValue = session.displayName
        nameLabel.toolTip = session.projectPath
        detailLabel.stringValue = detailText(for: session)
        refreshAge()
    }

    /// What the row says under the project name.
    ///
    /// A working session shows its activity; where the agent has not used a tool
    /// yet there is nothing honest to say, so it simply says `working`. An idle
    /// session never shows the last thing it did — a stale activity line reads as
    /// busy at a glance, which is the failure the pet exists to prevent.
    private func detailText(for session: AgentSession) -> String {
        switch session.state {
        case .working: return (session.activity ?? "Working").sentenceCased
        case .idle: return "Idle"
        case .unknown: return "State unknown"
        }
    }

    /// Seconds since this status was observed, recomputed every second.
    func refreshAge() {
        let seconds = max(0, Int(Date().timeIntervalSince(session.observedDate).rounded()))
        ageLabel.stringValue = seconds < 60
            ? "\(seconds)s"
            : "\(seconds / 60)m \(seconds % 60)s"
    }
}
