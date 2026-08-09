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
	presets: Preset[];
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

type AdvancedProps = {
	port: string;
	peer: string;
	onPort: (v: string) => void;
	onPeer: (v: string) => void;
	disabled: boolean;
	manual: string[];
	onAdd: (addr: string) => Promise<void>;
	onRemove: (addr: string) => Promise<void>;
};

type DiagnosticsProps = {
	stats: NetStats | null;
	running: boolean;
};

type DisclosureProps = {
	label: string;
	children: React.ReactNode;
};

type AlertProps = { message: string };
