/// Comparing the builds on the network, so a mismatch can be named.
///
/// Field by field as numbers, never as text: "1.10.0" is after "1.9.0" and
/// sorts before it as a string, and this app will reach a tenth minor release
/// long before anyone remembers this function exists.

const parts = (v: string) => {
	const bits = v.split('.').map(Number);
	const ok =
		bits.length === 3 && bits.every(n => Number.isInteger(n) && n >= 0);
	return ok ? bits : null;
};

/// -1, 0 or 1, and null when either side cannot be read.
///
/// Null rather than a guess. A machine running something this cannot parse is
/// not evidence that you are behind, and an app that tells somebody to update
/// on the strength of a string it did not understand has earned being ignored
/// the next time it says anything.
export const compare = (a: string, b: string) => {
	const [x, y] = [parts(a), parts(b)];
	if (!x || !y) return null;
	for (let i = 0; i < 3; i++) {
		if (x[i] !== y[i]) return x[i] > y[i] ? 1 : -1;
	}
	return 0;
};

/// Everyone here who is not on the same build as you, live only.
///
/// Live only because a version is the last thing a machine said before it went
/// quiet, and reporting the build of a PC that is switched off is remembering
/// rather than reporting — there is nothing to act on and it would keep the
/// warning on screen after the machine it names has gone.
export const mismatched = (peers: Peer[], mine: string) =>
	peers.filter(
		p => p.live && p.version && compare(p.version, mine) !== 0
	) as (Peer & { version: string })[];
