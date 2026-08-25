import AppKit

/// One live session.
///
/// Layout is: a state mark, which agent and which project, then the state or the
/// activity line, then a count-up of seconds since the status was observed. The
/// age is always on screen so staleness is visible directly rather than inferred
/// from a tuned threshold.
final class SessionRowView: NSView {
    private let indicator: StateIndicatorView
    private let nameLabel = Style.label("", font: Style.rowFont, color: Style.primary)
    private let detailLabel = Style.label("", font: Style.detailFont, color: Style.secondary)
    private let ageLabel = Style.label("", font: Style.detailFont, color: Style.secondary)

    private(set) var session: AgentSession

    init(session: AgentSession) {
        self.session = session
        indicator = StateIndicatorView(state: session.state)
        super.init(frame: .zero)

        ageLabel.alignment = .right
        ageLabel.setContentHuggingPriority(.required, for: .horizontal)
        ageLabel.setContentCompressionResistancePriority(.required, for: .horizontal)
        nameLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        detailLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        // Baseline, not centre. A project name that wraps to two lines made a
        // centred dot drift to the middle of the block, level with neither line;
        // aligning on the first baseline keeps the dot and the age beside the
        // first line however many lines the name takes.
        let top = NSStackView(views: [indicator, nameLabel, ageLabel])
        top.orientation = .horizontal
        top.distribution = .fill
        top.alignment = .firstBaseline
        top.spacing = 6
        NSLayoutConstraint.activate([
            indicator.widthAnchor.constraint(equalToConstant: StateIndicatorView.size),
            indicator.heightAnchor.constraint(equalToConstant: StateIndicatorView.size),
        ])

        // The detail line hangs under the project name rather than under the
        // mark, so the mark stays the only thing in the row's left margin and
        // reads as a column down the surface when several sessions are live.
        let detailRow = NSStackView(views: [detailLabel])
        detailRow.orientation = .horizontal
        detailRow.edgeInsets = NSEdgeInsets(
            top: 0, left: StateIndicatorView.size + 6, bottom: 0, right: 0
        )

        let stack = NSStackView(views: [top, detailRow])
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
            detailRow.widthAnchor.constraint(equalTo: stack.widthAnchor),
        ])

        apply(session)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func apply(_ session: AgentSession) {
        self.session = session
        indicator.apply(session.state)
        nameLabel.attributedStringValue = title(for: session)
        nameLabel.toolTip = session.projectPath
        detailLabel.stringValue = detailText(for: session)
        refreshAge()
    }

    /// One frame of the surface's one-second clock.
    ///
    /// Only the age needs it. The indicator breathes in the render server and
    /// keeps its own time.
    func tick() {
        refreshAge()
    }

    /// Which agent, then which project.
    ///
    /// The agent is drawn from whatever `agentId` the core sent, so the pet holds
    /// no table of agent names and adding an agent changes nothing here. It sits
    /// in the secondary colour because the project is what the eye is looking
    /// for; the agent only has to be legible without a click.
    private func title(for session: AgentSession) -> NSAttributedString {
        let title = NSMutableAttributedString(
            string: session.agentId.sentenceCased + " ",
            attributes: [.font: Style.rowFont, .foregroundColor: Style.secondary]
        )
        title.append(NSAttributedString(
            string: session.displayName,
            attributes: [.font: Style.rowFont, .foregroundColor: Style.primary]
        ))
        return title
    }

    /// What the row says under the project name.
    ///
    /// A working session shows its activity; where the agent has not used a tool
    /// yet there is nothing honest to say, so it simply says `working`. An idle
    /// session never shows the last thing it did — a stale activity line reads as
    /// busy at a glance, which is the failure the pet exists to prevent.
    ///
    /// Waiting and errored both arrive with their line already composed by the
    /// core: the agent's own words for what it is blocked on, or the status code
    /// it stopped with. Neither is invented here, and neither falls back to
    /// something more confident than the core was able to say.
    private func detailText(for session: AgentSession) -> String {
        switch session.state {
        case .working: return (session.activity ?? "Working").sentenceCased
        case .idle: return "Idle"
        case .waiting: return (session.activity ?? "Waiting for you").sentenceCased
        case .errored: return (session.activity ?? "Errored").sentenceCased
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
