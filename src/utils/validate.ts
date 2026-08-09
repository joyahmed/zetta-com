/// Validated here rather than at the IPC boundary so a typo reads as a
/// sentence, instead of surfacing as a serialisation failure about `u16`.
export const parsePort = (value: string): number | string => {
	const port = Number(value);
	if (!Number.isInteger(port) || port < 1 || port > 65535) {
		return 'Port must be a whole number between 1 and 65535.';
	}
	return port;
};

export const isPortError = (result: number | string): result is string =>
	typeof result === 'string';
