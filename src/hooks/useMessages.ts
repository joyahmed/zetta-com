import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

const POLL_MS = 600;

/// Text is its own hook for the same reason it is its own command: in v1 voice
/// and text went out on one keypress, and when only one of them arrived there
/// was no way to tell which half had failed.
export const useMessages = (running: boolean) => {
	const [messages, setMessages] = useState<Message[]>([]);

	useEffect(() => {
		if (!running) {
			setMessages([]);
			return;
		}
		let alive = true;
		const id = setInterval(async () => {
			try {
				const m = await invoke<Message[]>('messages');
				if (alive) setMessages(m);
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
