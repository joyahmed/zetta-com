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

/// Someone found on the LAN. `live` goes false when nothing has been heard from
/// them for a while — they stay in the roster rather than vanishing, because
/// "was here, now gone" tells you more than a name quietly disappearing.
type Peer = {
	id: string;
	name: string;
	addr: string;
	live: boolean;
	/// Audio arrived from them in the last fraction of a second. With
	/// push-to-talk, receiving audio *is* the fact that somebody is speaking —
	/// no flag in the header could say it more reliably.
	talking: boolean;
};

type TalkBarProps = { held: boolean; key_: string };

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
	selected: string;
	onSelect: (addr: string) => void;
};

type AdvancedProps = {
	port: string;
	peer: string;
	onPort: (v: string) => void;
	onPeer: (v: string) => void;
	disabled: boolean;
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
