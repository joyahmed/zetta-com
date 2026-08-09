/// A short two-note chime for a message that arrived while you were not
/// looking.
///
/// A toast is not enough on its own. It is gone in a few seconds whether or not
/// anybody saw it, and when the app is hidden to the tray there is not even a
/// taskbar button to flash — so the one cue that survives being ignored is one
/// you hear. This is a tool for reaching people who are doing something else.
///
/// Synthesised rather than shipped as a file: two oscillators cost nothing, and
/// an asset would have to be decoded, bundled, and chosen — and every sound
/// anybody picks is wrong for somebody's office.
///
/// Played through the webview, which means the system default output rather
/// than the device chosen in Settings. That is deliberate: the intercom's own
/// output may be a headset lying on a desk, and a notification you cannot hear
/// is the thing being fixed.
let ctx: AudioContext | null = null;

export const chime = () => {
	try {
		// Created on first use, not at import. A context made before any user
		// gesture starts suspended, and it would then be permanently silent.
		ctx ??= new AudioContext();
		if (ctx.state === 'suspended') void ctx.resume();

		const now = ctx.currentTime;
		// Two rising notes. A single tone reads as an error sound; a rise reads
		// as an arrival, which is what this is.
		for (const [at, hz] of [
			[0, 880],
			[0.12, 1175]
		] as const) {
			const osc = ctx.createOscillator();
			const gain = ctx.createGain();
			osc.type = 'sine';
			osc.frequency.value = hz;

			// Ramped, never switched. A gain that jumps from 0 produces a click
			// at the discontinuity, which is louder and more annoying than the
			// note itself.
			gain.gain.setValueAtTime(0.0001, now + at);
			gain.gain.exponentialRampToValueAtTime(0.18, now + at + 0.015);
			gain.gain.exponentialRampToValueAtTime(0.0001, now + at + 0.16);

			osc.connect(gain).connect(ctx.destination);
			osc.start(now + at);
			osc.stop(now + at + 0.18);
		}
	} catch {
		// No audio context, or the webview refused one. The notification and the
		// unread mark still stand; this was the part that carries across a room.
	}
};
