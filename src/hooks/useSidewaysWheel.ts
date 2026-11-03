import { useEffect, useRef } from 'react';

/// Lets an ordinary mouse wheel scroll a sideways strip.
///
/// A wheel only sends vertical deltas, and a row that overflows to the right
/// ignores them, so with a mouse the chips past the edge were unreachable — the
/// Send-to row hid every PC after the sixth or so. A trackpad already sends a
/// horizontal delta; that one is left alone so two-finger swipes feel native.
///
/// Only takes the wheel while the strip can actually move in that direction, so
/// at either end the event falls through and the page behind it still scrolls.
/// The listener is added by hand because React's `onWheel` is passive and
/// cannot call `preventDefault`.
export const useSidewaysWheel = <T extends HTMLElement>() => {
	const ref = useRef<T>(null);

	useEffect(() => {
		const el = ref.current;
		if (!el) return;

		const onWheel = (e: WheelEvent) => {
			if (Math.abs(e.deltaX) >= Math.abs(e.deltaY)) return;
			const max = el.scrollWidth - el.clientWidth;
			if (max <= 0) return;
			// deltaMode 1 is lines (Firefox, some Windows mice): one line ~ one chip-ish.
			const dy = e.deltaMode === 1 ? e.deltaY * 32 : e.deltaY;
			const next = Math.min(max, Math.max(0, el.scrollLeft + dy));
			if (next === el.scrollLeft) return;
			e.preventDefault();
			el.scrollLeft = next;
		};

		el.addEventListener('wheel', onWheel, { passive: false });
		return () => el.removeEventListener('wheel', onWheel);
	}, []);

	return ref;
};
