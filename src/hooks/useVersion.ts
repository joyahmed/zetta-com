import { getVersion } from '@tauri-apps/api/app';
import { useEffect, useState } from 'react';

/// This build's own version, to compare against what the roster reports. Read
/// once — it cannot change while the app is running.
///
/// Taken from Tauri rather than from package.json, and that matters for more
/// than tidiness: `getVersion` returns the crate's own `CARGO_PKG_VERSION`,
/// which is the exact string the heartbeat puts on the wire. Both sides of the
/// comparison therefore come from one place. Reading package.json instead would
/// have compared two numbers that are only equal because somebody remembered to
/// bump both, and the failure would have been every machine on the network
/// accusing every other one of being out of step.
export const useVersion = () => {
	const [version, setVersion] = useState('');

	useEffect(() => {
		getVersion()
			.then(setVersion)
			.catch(() => {
				// Empty compares as unreadable, so the roster simply says
				// nothing about builds. Failing quiet is right here: this is a
				// diagnostic, and a diagnostic that itself errors is noise.
			});
	}, []);

	return version;
};
