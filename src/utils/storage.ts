const STORE_KEY = 'zc.transport';

const DEFAULTS: Saved = { port: '9001', peer: '' };

/// Never throws. Storage can be unreadable, disabled, or hold something written
/// by an older build, and none of those are worth failing a launch over — the
/// defaults are always a working starting point.
export const loadSaved = (): Saved => {
	try {
		const raw = localStorage.getItem(STORE_KEY);
		if (raw) return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<Saved>) };
	} catch {
		// Fall through to defaults.
	}
	return DEFAULTS;
};

export const saveSaved = (saved: Saved) => {
	try {
		localStorage.setItem(STORE_KEY, JSON.stringify(saved));
	} catch {
		// Losing a preference is not worth surfacing to anyone.
	}
};
