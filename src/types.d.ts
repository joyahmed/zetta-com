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
	/// The shared room passphrase. Empty means nothing is encrypted.
	passphrase: string;
	/// Chosen microphone and speakers by name. Null means the system default.
	inputDevice: string | null;
	outputDevice: string | null;
	presets: Preset[];
	/// Seconds a login start waits before binding. Zero means no wait.
	startDelay: number;
	/// Keep hearing others while talking. Only safe on headphones.
	headphones: boolean;
	sounds: Sounds;
	/// Addresses whose going offline and coming back plays a tone.
	watched: string[];
	groups: Group[];
};

/// A saved set of PCs to address together. Members by address.
type Group = { name: string; members: string[] };

type GroupsProps = {
	groups: Group[];
	peers: Peer[];
	onSave: (next: Group[]) => Promise<void>;
};

/// One switch per sound, one volume (0 to 1) for all of them.
type Sounds = {
	message: boolean;
	voice: boolean;
	key: boolean;
	presence: boolean;
	volume: number;
};

type SoundsProps = {
	sounds: Sounds;
	onChoose: (next: Partial<Sounds>) => Promise<void>;
};

type StartupProps = {
	on: boolean;
	onChoose: (next: boolean) => void;
	delay: string;
	onDelay: (v: string) => void;
	onCommit: () => void;
};

type RoomProps = {
	passphrase: string;
	/// Short code derived from the key, for comparing across machines. Null
	/// when there is no passphrase.
	code: string | null;
	onGenerate: () => Promise<void>;
	/// An empty passphrase leaves the room and goes back to the clear.
	onJoin: (passphrase: string) => Promise<void>;
};

type DevicesProps = {
	inputs: string[];
	outputs: string[];
	input: string;
	output: string;
	onChoose: (next: { input?: string; output?: string }) => Promise<void>;
	onRefresh: () => Promise<void>;
	headphones: boolean;
	onHeadphones: (on: boolean) => Promise<void>;
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
	/// The build they are running, once they have said so. Null for a machine
	/// that has not sent one yet, and for every build older than this feature,
	/// which never sends one at all — so null is an ordinary state and never
	/// means anything is wrong.
	version: string | null;
	/// Do-not-disturb is on at their end: text still reaches them, voice is not
	/// played. False for a build too old to say, which has no DND to be on.
	busy: boolean;
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
	/// Your own lines only: who it went to, and who has answered that it
	/// arrived. Names where the roster knows them. A machine on a build older
	/// than receipts never answers, so a short `heard` is not a failure.
	to: string[];
	heard: string[];
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
	groups: Group[];
	running: boolean;
	/// null for everyone, `group:<name>` for a group, otherwise an address.
	target: string | null;
	onTarget: (addr: string | null) => void;
	onSeeAll: () => void;
};

type FieldProps = {
	label: string;
	value: string;
	onChange: (v: string) => void;
	/// For a value worth committing once rather than on every keystroke — a
	/// number being typed passes through smaller numbers on the way.
	onBlur?: () => void;
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

/// The machines not on your build, and your own version to name beside them.
/// `version` is non-null here because the list is filtered on it.
type BuildsProps = {
	odd: (Peer & { version: string })[];
	mine: string;
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
	/// Set for keys that can be rebound; null for the generated per-PC ranges.
	id: string | null;
};

type ShortcutsProps = {
	shortcuts: ShortcutInfo[];
	/// Change one key. An empty spec puts it back to its default.
	onSet: (id: string, spec: string) => Promise<void>;
};

type NavProps = {
	me: string;
	running: boolean;
	onToggle: () => void;
	onAddPc: () => void;
	onShortcuts: () => void;
	onSettings: () => void;
	onDiagnostics: () => void;
	/// Do-not-disturb, this machine. Switched here or from the tray.
	dnd: boolean;
	onDnd: (on: boolean) => void;
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
	/// The full roster in the order wanted, by address. This decides which PC
	/// each Ctrl+n reaches.
	onReorder: (order: string[]) => Promise<void>;
	/// Addresses that play a tone when they go offline or come back.
	watched: string[];
	onWatch: (addr: string, on: boolean) => Promise<void>;
};

type ModalProps = {
	title: string;
	open: boolean;
	onClose: () => void;
	children: React.ReactNode;
};

type AlertProps = { message: string };
