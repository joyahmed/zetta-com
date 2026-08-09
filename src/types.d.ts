type NetStats = {
	port: number;
	peer: string;
	tx: number;
	rx: number;
	bad: number;
	lost: number;
	lastSeq: number;
};

type Saved = { port: string; peer: string };

/// Mirrors the Rust side's transport.json. The transport reads this before any
/// window exists, which is why it is not localStorage.
type Preset = { label: string; text: string; shortcut: string };

type Config = {
	port: number;
	peer: string;
	manual: string[];
	talkShortcut: string;
	/// Chosen microphone and speakers by name. Null means the system default.
	inputDevice: string | null;
	outputDevice: string | null;
	presets: Preset[];
};

type DevicesProps = {
	inputs: string[];
	outputs: string[];
	input: string;
	output: string;
	onChoose: (next: { input?: string; output?: string }) => Promise<void>;
	onRefresh: () => Promise<void>;
};

type PresetsProps = {
	presets: Preset[];
	onSend: (text: string) => void;
	disabled: boolean;
};

/// Someone found on the LAN. `live` goes false when nothing has been heard from
/// them for a while — they stay in the roster rather than vanishing, because
/// "was here, now gone" tells you more than a name quietly disappearing.
type Peer = {
	id: string;
	name: string;
	addr: string;
	live: boolean;
	/// Typed in rather than discovered. A manual entry that never goes live is a
	/// wrong address; a discovered one that goes quiet is a switched-off PC.
	manual: boolean;
	/// Audio arrived from them in the last fraction of a second. With
	/// push-to-talk, receiving audio *is* the fact that somebody is speaking —
	/// no flag in the header could say it more reliably.
	talking: boolean;
};

type TalkBarProps = { held: boolean; key_: string; to: string };

type Message = {
	id: number;
	/// A name once the session has matched the address to the roster; the raw
	/// address when it could not. Empty for your own lines.
	from: string;
	text: string;
	mine: boolean;
	/// Unix milliseconds.
	at: number;
};

type MessagesProps = {
	to: string;
	messages: Message[];
	onSend: (text: string) => void;
	disabled: boolean;
	/// Rendered between the log and the input — the presets, in practice. A
	/// slot rather than a presets prop, because the log has no business knowing
	/// what a preset is; it only owns the fact that something sits there.
	quick?: React.ReactNode;
};

/// The one-row target strip on the home screen. `onSeeAll` opens the full
/// roster, which is where addresses and liveness are legible.
type TargetsProps = {
	peers: Peer[];
	running: boolean;
	target: string | null;
	onTarget: (addr: string | null) => void;
	onSeeAll: () => void;
};

type FieldProps = {
	label: string;
	value: string;
	onChange: (v: string) => void;
	disabled?: boolean;
	placeholder?: string;
	className?: string;
};

type StatProps = { label: string; value?: number; warn?: boolean };

type DotProps = { on: boolean };

type PeerRowProps = {
	slot?: number;
	peer: Peer;
	selected: boolean;
	onSelect: () => void;
};

type RosterProps = {
	peers: Peer[];
	running: boolean;
	/// The address everything is aimed at, or null for everyone.
	target: string | null;
	onTarget: (addr: string | null) => void;
};

type ConnectionProps = {
	port: string;
	onPort: (v: string) => void;
	disabled: boolean;
};

type AddPcProps = {
	onAdd: (addr: string) => Promise<void>;
	/// Applied straight after the add, keyed on the address as typed — which is
	/// how a manual entry is identified everywhere until discovery resolves it.
	onName: (addr: string, label: string) => Promise<void>;
};

type DiagnosticsProps = {
	stats: NetStats | null;
	running: boolean;
};

type ShortcutInfo = {
	label: string;
	keys: string;
	/// False when another application already owns the combination. The key
	/// then does nothing at all, which is why this is shown rather than logged.
	registered: boolean;
};

type ShortcutsProps = { shortcuts: ShortcutInfo[] };

type NavProps = {
	me: string;
	running: boolean;
	onToggle: () => void;
	onAddPc: () => void;
	onShortcuts: () => void;
	onSettings: () => void;
	onDiagnostics: () => void;
};

/// One line of the PCs list. Flattened from a discovered `Peer` or from a bare
/// manual address, which has no roster entry at all while the transport is
/// stopped but still has to be editable.
type PcRow = {
	addr: string;
	name: string;
	manual: boolean;
	live: boolean;
	/// 1–9 where this PC has a `Ctrl+n` key, absent otherwise. Counted over the
	/// discovered roster only, so it matches what the chips and the All PCs
	/// list show — a manual entry the transport has not picked up yet is in
	/// this list but not in the roster the shortcuts index into.
	slot?: number;
};

type PcsProps = {
	peers: Peer[];
	manual: string[];
	onRename: (addr: string, label: string) => Promise<void>;
	onEdit: (from: string, to: string) => Promise<void>;
	onRemove: (addr: string) => Promise<void>;
};

type ModalProps = {
	title: string;
	open: boolean;
	onClose: () => void;
	children: React.ReactNode;
};

type AlertProps = { message: string };
