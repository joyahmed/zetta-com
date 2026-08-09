import { invoke } from '@tauri-apps/api/core';
import {
	isPermissionGranted,
	requestPermission,
	sendNotification
} from '@tauri-apps/plugin-notification';
import { useEffect, useRef, useState } from 'react';
import { DEMO, DEMO_MESSAGES } from '../demo';

const POLL_MS = 600;

/// Text is its own hook for the same reason it is its own command: in v1 voice
/// and text went out on one keypress, and when only one of them arrived there
/// was no way to tell which half had failed.
export const useMessages = (running: boolean) => {
	const [messages, setMessages] = useState<Message[]>(
		DEMO ? DEMO_MESSAGES : []
	);
	// Highest id already seen. Notifying on anything above it means a restart
	// never replays the whole log at you as a burst of toasts.
	const seen = useRef(0);
	const allowed = useRef(false);

	useEffect(() => {
		(async () => {
			allowed.current =
				(await isPermissionGranted()) ||
				(await requestPermission()) === 'granted';
		})().catch(() => {});
	}, []);

	useEffect(() => {
		if (DEMO) return;
		if (!running) {
			setMessages([]);
			return;
		}
		let alive = true;
		const id = setInterval(async () => {
			try {
				const m = await invoke<Message[]>('messages');
				if (!alive) return;
				setMessages(m);

				// Only what arrived, and only when the window is not being
				// looked at. This app lives in the tray; a notification for a
				// message already on screen is noise.
				if (allowed.current && document.hidden) {
					for (const msg of m) {
						if (msg.id > seen.current && !msg.mine) {
							sendNotification({
								title: msg.from || 'Intercom',
								body: msg.text
							});
						}
					}
				}
				seen.current = Math.max(seen.current, ...m.map(x => x.id), 0);
			} catch {
				// The transport reports its own failures; a missed poll here
				// would only duplicate them.
			}
		}, POLL_MS);
		return () => {
			alive = false;
			clearInterval(id);
		};
	}, [running]);

	const send = async (text: string) => {
		await invoke('send_text', { text });
	};

	return { messages, send };
};
