import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { DEMO, DEMO_DEVICES } from '../demo';

/// The microphones and speakers this machine has, and which two are chosen.
///
/// Listed once at mount rather than polled: enumerating devices is a blocking
/// call into the audio host, and doing it on a timer alongside the roster poll
/// would be a lot of work to notice something that changes a few times a day.
/// The refresh button is there for when it does.
export const useDevices = (onError: (message: string) => void) => {
	const [inputs, setInputs] = useState<string[]>([]);
	const [outputs, setOutputs] = useState<string[]>([]);
	const [input, setInput] = useState('');
	const [output, setOutput] = useState('');

	const list = async () => {
		if (DEMO) {
			setInputs(DEMO_DEVICES.inputs);
			setOutputs(DEMO_DEVICES.outputs);
			return;
		}
		try {
			const [ins, outs] = await invoke<[string[], string[]]>('audio_devices');
			setInputs(ins);
			setOutputs(outs);
		} catch (e) {
			onError(String(e));
		}
	};

	useEffect(() => {
		list();
		if (DEMO) {
			setInput(DEMO_DEVICES.input);
			setOutput(DEMO_DEVICES.output);
			return;
		}
		invoke<Config | null>('config_get')
			.then(c => {
				setInput(c?.inputDevice ?? '');
				setOutput(c?.outputDevice ?? '');
			})
			.catch(() => {});
	}, []);

	/// Empty means "system default", not a device named "". Sent as null so the
	/// config stores an absence rather than a name that can never match.
	const choose = async (next: { input?: string; output?: string }) => {
		const nextIn = next.input ?? input;
		const nextOut = next.output ?? output;
		setInput(nextIn);
		setOutput(nextOut);
		if (DEMO) return;
		try {
			await invoke('set_audio_devices', {
				input: nextIn || null,
				output: nextOut || null
			});
		} catch (e) {
			onError(String(e));
		}
	};

	return { inputs, outputs, input, output, choose, refresh: list };
};
