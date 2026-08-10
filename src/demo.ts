/// Stand-in data for screenshots.
///
/// A screenshot of an empty roster and an empty log shows nothing about what the
/// app is for, and staging a real one needs four machines switched on at once.
///
/// **This cannot reach a release build.** `import.meta.env.VITE_DEMO` is
/// substituted by Vite at build time, so with the variable unset `DEMO` is the
/// literal `false`, every branch guarded by it is dead code, and the arrays
/// below are dropped from the bundle entirely. Run it deliberately:
///
/// ```powershell
/// $env:VITE_DEMO = '1'; bun run tauri dev
/// ```
export const DEMO = import.meta.env.VITE_DEMO === '1';

/// Three present, one gone. The absent one is the point: reporting that
/// somebody is *not* there is the thing v1 could never do, and a roster where
/// everybody is green demonstrates the half of it that everything else does too.
/// Addresses ascend down the list so the slot numbers and the addresses read in
/// the same direction — in a screenshot, two columns that disagree about their
/// ordering look like a bug rather than a roster.
export const DEMO_PEERS: Peer[] = [
	{
		id: 'demo-fatema',
		name: 'Fatema',
		addr: '192.168.0.118:9001',
		live: true,
		talking: false,
		manual: false,
		version: null
	},
	{
		id: 'demo-riya',
		name: 'Riya',
		addr: '192.168.0.126:9001',
		live: true,
		talking: false,
		manual: false,
		version: null
	},
	{
		id: 'demo-salman',
		name: 'Salman',
		addr: '192.168.0.142:9001',
		live: true,
		talking: true,
		manual: false,
		version: null
	},
	{
		id: 'demo-emon',
		name: 'Emon',
		addr: '192.168.0.151:9001',
		live: true,
		talking: false,
		manual: false,
		version: null
	},
	{
		id: 'demo-ahad',
		name: 'Ahad',
		addr: '192.168.0.159:9001',
		live: true,
		talking: false,
		manual: false,
		version: null
	},
	{
		id: 'demo-rashique',
		name: 'Rashique',
		addr: '192.168.0.167:9001',
		live: false,
		talking: false,
		manual: false,
		version: null
	},
	{
		// Manually added, so the Settings list has one row with an editable
		// address in it rather than seven that all came from discovery.
		id: 'demo-himel',
		name: 'Himel',
		addr: '192.168.0.174:9001',
		live: true,
		talking: false,
		manual: true,
		version: null
	}
];

export const DEMO_STATS: NetStats = {
	port: 9001,
	peer: '',
	tx: 184203,
	rx: 209117,
	bad: 0,
	lost: 12,
	lastSeq: 41288
};

/// Fixed timestamps rather than something derived from the clock, so the same
/// screenshot taken twice is the same screenshot.
const at = (hh: number, mm: number) => new Date(2026, 0, 1, hh, mm).getTime();

/// Everybody in the roster says something, and Rashique says the thing that
/// explains why he is grey by the time the screenshot is taken — the log and the
/// roster should tell the same story rather than two unrelated ones.
/// Kept short enough that no line wraps and the whole exchange fits the log
/// without scrolling. A screenshot with a half-cut sentence at the bottom edge
/// reads as a broken layout, whatever it actually says.
export const DEMO_MESSAGES: Message[] = [
	{ id: 1, from: '', text: 'Freeze on main until the release is out', mine: true, at: at(9, 10) },
	{ id: 2, from: 'Salman', text: 'CI green on my branch', mine: false, at: at(9, 12) },
	{ id: 3, from: '', text: 'Hold, staging is on the old migration', mine: true, at: at(9, 13) },
	{ id: 4, from: 'Rashique', text: 'Off to the client site, shutting down', mine: false, at: at(9, 18) },
	{ id: 5, from: '', text: 'Push your branch before you go', mine: true, at: at(9, 18) },
	{ id: 6, from: 'Himel', text: '500s on /api/payroll in staging', mine: false, at: at(9, 26) },
	{ id: 7, from: '', text: 'Ahad, you touched it last. Take it', mine: true, at: at(9, 27) },
	{ id: 8, from: 'Ahad', text: 'On it', mine: false, at: at(9, 27) },
	{ id: 9, from: 'Fatema', text: 'PR 412 needs a second review', mine: false, at: at(9, 30) },
	{ id: 10, from: 'Riya', text: 'I will take it', mine: false, at: at(9, 31) },
	{ id: 11, from: 'Emon', text: 'Migration done, new schema is up', mine: false, at: at(9, 34) },
	{ id: 12, from: '', text: 'Run the smoke tests before anyone merges', mine: true, at: at(9, 35) },
	{ id: 13, from: 'Salman', text: 'Smoke passed, deploying', mine: false, at: at(9, 41) }
];

export const DEMO_ROOM = {
	passphrase: 'cedar-harbor-quartz-thistle-ivory-9f4c2ab7e1d05836',
	code: '7B2E'
};

export const DEMO_DEVICES = {
	inputs: [
		'Headset Microphone (Jabra Evolve 40)',
		'Microphone Array (Realtek Audio)',
		'Webcam (Logitech C920)'
	],
	outputs: [
		'Headset Earphone (Jabra Evolve 40)',
		'Speakers (Realtek Audio)',
		'DELL U2520D (NVIDIA High Definition Audio)'
	],
	input: 'Headset Microphone (Jabra Evolve 40)',
	output: 'Headset Earphone (Jabra Evolve 40)'
};

export const DEMO_PRESETS: Preset[] = [
	{ label: 'On my way', text: 'On my way', shortcut: '' },
	{ label: 'Need you here', text: 'Need you here', shortcut: '' },
	{ label: 'Give me 5', text: 'Give me 5 minutes', shortcut: '' }
];
