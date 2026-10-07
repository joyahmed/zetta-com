/// Every sound the app makes, apart from people's voices.
///
/// Synthesised rather than shipped as files: a couple of oscillators cost
/// nothing, and an asset would have to be decoded, bundled, and chosen — and
/// every sound anybody picks is wrong for somebody's office.
///
/// Played through the webview, which means the system default output rather
/// than the device chosen in Settings. That is deliberate: the intercom's own
/// output may be a headset lying on a desk, and a notification you cannot hear
/// is the thing being fixed.
///
/// Each sound is a shape, not just a pitch, so they can be told apart without
/// looking: a rise is an arrival, a fall is a departure, a single blip is "a
/// voice is coming", and a tick is your own key.

type Note = { at: number; hz: number; len: number };

/// Which sounds play and how loud. Set from the saved config by `useSounds`;
/// these are the same defaults Rust writes for a config that has none.
let prefs: Sounds = {
	message: true,
	voice: true,
	key: false,
	presence: true,
	volume: 0.7
};

export const setSoundPrefs = (next: Sounds) => {
	prefs = next;
};

/// Do-not-disturb. Above every per-sound switch: it is one switch for "leave
/// me alone", and a chime that still played because its own switch was on
/// would make it a liar. Set by `useDnd`; the toast in `useMessages` asks it
/// too. Not saved — see `audio::DND` in Rust.
let quiet = false;

export const setQuiet = (on: boolean) => {
	quiet = on;
};

export const isQuiet = () => quiet;

let ctx: AudioContext | null = null;

/// `level` scales one sound against the others: a key tick should sit well
/// under a chime meant to reach across a room.
const play = (notes: Note[], level = 1) => {
	try {
		// Created on first use, not at import. A context made before any user
		// gesture starts suspended, and it would then be permanently silent.
		ctx ??= new AudioContext();
		if (ctx.state === 'suspended') void ctx.resume();

		// 0.25 at full volume. The default 0.7 lands on 0.175, which is what the
		// message chime has always played at.
		const peak = 0.25 * prefs.volume * level;
		if (peak <= 0) return;
		const now = ctx.currentTime;
		for (const { at, hz, len } of notes) {
			const osc = ctx.createOscillator();
			const gain = ctx.createGain();
			osc.type = 'sine';
			osc.frequency.value = hz;

			// Ramped, never switched. A gain that jumps from 0 produces a click
			// at the discontinuity, which is louder and more annoying than the
			// note itself. exponentialRamp cannot reach 0, hence 0.0001.
			gain.gain.setValueAtTime(0.0001, now + at);
			gain.gain.exponentialRampToValueAtTime(peak, now + at + 0.01);
			gain.gain.exponentialRampToValueAtTime(0.0001, now + at + len);

			osc.connect(gain).connect(ctx.destination);
			osc.start(now + at);
			osc.stop(now + at + len + 0.02);
		}
	} catch {
		// No audio context, or the webview refused one. Whatever this sound was
		// announcing is still on screen.
	}
};

const SHAPES = {
	// Two rising notes. A single tone reads as an error sound; a rise reads as
	// an arrival, which is what this is.
	message: () =>
		play([
			{ at: 0, hz: 880, len: 0.16 },
			{ at: 0.12, hz: 1175, len: 0.16 }
		]),
	// One short high blip. It plays as the voice starts, not before it — a
	// beep that came first would mean delaying every voice by its length.
	voice: () => play([{ at: 0, hz: 1320, len: 0.08 }], 0.8),
	// A tick down and a lower tick up, short and quiet. On speakers the
	// microphone is already open when the down tick plays, so it reaches the
	// room; that is why the setting is off by default.
	keyDown: () => play([{ at: 0, hz: 1600, len: 0.03 }], 0.35),
	keyUp: () => play([{ at: 0, hz: 1100, len: 0.03 }], 0.35),
	// Falling for gone, rising for back — the same pair of notes either way.
	offline: () =>
		play([
			{ at: 0, hz: 784, len: 0.18 },
			{ at: 0.15, hz: 523, len: 0.26 }
		]),
	online: () =>
		play([
			{ at: 0, hz: 523, len: 0.16 },
			{ at: 0.13, hz: 784, len: 0.2 }
		])
};

export type SoundName = keyof typeof SHAPES;

/// Which switch in Settings governs each sound.
const SWITCH: Record<SoundName, keyof Omit<Sounds, 'volume'>> = {
	message: 'message',
	voice: 'voice',
	keyDown: 'key',
	keyUp: 'key',
	offline: 'presence',
	online: 'presence'
};

/// Play a sound if its switch is on and do-not-disturb is off.
export const sound = (name: SoundName) => {
	if (!quiet && prefs[SWITCH[name]]) SHAPES[name]();
};

/// Play a sound whatever its switch says, for the Test buttons. Volume still
/// applies: hearing it at the level it will really play is the point.
export const preview = (name: SoundName) => SHAPES[name]();
