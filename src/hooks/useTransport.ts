import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import { loadSaved, saveSaved } from '../utils/storage';
import { isPortError, parsePort } from '../utils/validate';

/// How often the UI asks Rust what is happening. Deliberately a poll rather
/// than an event per packet: audio runs at fifty packets a second per peer, and
/// an IPC message each would melt the webview. Polling also decouples the
/// redraw rate from the packet rate without needing a throttle.
const POLL_MS = 400;

export const useTransport = () => {
	const saved = useRef(loadSaved()).current;
	const [port, setPort] = useState(saved.port);
	const [peer, setPeer] = useState(saved.peer);
	const [stats, setStats] = useState<NetStats | null>(null);
	const [peers, setPeers] = useState<Peer[]>([]);
	const [error, setError] = useState('');

	useEffect(() => {
		saveSaved({ port, peer });
	}, [port, peer]);

	// One poll answers both questions: net_stats returns null exactly when the
	// transport is stopped, so there is no separate "is it running" call that
	// could drift out of agreement with reality.
	useEffect(() => {
		let alive = true;
		const id = setInterval(async () => {
			try {
				const [s, p] = await Promise.all([
					invoke<NetStats | null>('net_stats'),
					invoke<Peer[]>('net_peers')
				]);
				if (!alive) return;
				setStats(s);
				setPeers(p);
			} catch (e) {
				if (alive) setError(String(e));
			}
		}, POLL_MS);
		// Without this, every re-render stacks another interval and the poll
		// rate quietly climbs until the webview is doing nothing else.
		return () => {
			alive = false;
			clearInterval(id);
		};
	}, []);

	const start = async () => {
		setError('');
		const parsed = parsePort(port);
		if (isPortError(parsed)) {
			setError(parsed);
			return;
		}
		// An address is optional now: discovery finds everyone on a normal
		// network, and Advanced exists for the ones that filter mDNS.
		try {
			await invoke('net_start', { port: parsed, peer });
		} catch (e) {
			setError(String(e));
		}
	};

	const stop = async () => {
		setError('');
		try {
			await invoke('net_stop');
			setStats(null);
			setPeers([]);
		} catch (e) {
			setError(String(e));
		}
	};

	return {
		port,
		setPort,
		peer,
		setPeer,
		stats,
		peers,
		error,
		// Exposed so sibling hooks can report through the one banner rather
		// than each growing an error surface of its own.
		setError,
		start,
		stop,
		running: stats !== null
	};
};
