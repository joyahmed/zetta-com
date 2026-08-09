import { useState } from 'react';
import { Advanced } from './components/Advanced';
import { Alert } from './components/Alert';
import { Diagnostics } from './components/Diagnostics';
import { Messages } from './components/Messages';
import { Modal } from './components/Modal';
import { Nav } from './components/Nav';
import { Presets } from './components/Presets';
import { Roster } from './components/Roster';
import { Shortcuts } from './components/Shortcuts';
import { TalkBar } from './components/TalkBar';
import { useLocalName } from './hooks/useLocalName';
import { useManualPeers } from './hooks/useManualPeers';
import { useMessages } from './hooks/useMessages';
import { usePtt } from './hooks/usePtt';
import { useShortcuts } from './hooks/useShortcuts';
import { useTarget } from './hooks/useTarget';
import { useTransport } from './hooks/useTransport';

const App = () => {
	const me = useLocalName();
	const held = usePtt();
	const {
		port,
		setPort,
		peer,
		setPeer,
		stats,
		peers,
		error,
		setError,
		start,
		stop,
		running
	} = useTransport();
	const { messages, send } = useMessages(running);
	const { manual, presets, add, remove } = useManualPeers(setError);
	const { target, setTarget } = useTarget(setError);
	const {
		shortcuts,
		showShortcuts,
		setShowShortcuts,
		showAddPc,
		setShowAddPc
	} = useShortcuts();
	const [showSettings, setShowSettings] = useState(false);

	// Named for the talk bar and the message box, so it always says who is
	// about to be addressed rather than leaving it to be remembered.
	const targetName =
		target === null
			? 'everyone'
			: (peers.find(p => p.addr === target)?.name ?? target);

	return (
		<div className='flex min-h-screen flex-col bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100'>
			<Nav
				me={me}
				running={running}
				onToggle={running ? stop : start}
				onAddPc={() => setShowAddPc(true)}
				onShortcuts={() => setShowShortcuts(true)}
				onSettings={() => setShowSettings(true)}
				{...{
					me
				}}
			/>

			{/* The main screen is the two things this app is: who you are
			    talking to, and what has been said. Everything else is behind
			    the bar. */}
			<main className='flex flex-1 flex-col gap-4 p-4'>
				{error && <Alert message={error} />}

				{running && <TalkBar held={held} key_='F8' to={targetName} />}

				<Roster
					peers={peers}
					running={running}
					target={target}
					onTarget={setTarget}
				/>

				<Presets presets={presets} onSend={send} disabled={!running} />

				<Messages
					messages={messages}
					onSend={send}
					disabled={!running}
					to={targetName}
				/>
			</main>

			<Modal
				title='Shortcuts'
				open={showShortcuts}
				onClose={() => setShowShortcuts(false)}
			>
				<Shortcuts shortcuts={shortcuts} />
			</Modal>

			<Modal
				title='Add a PC'
				open={showAddPc}
				onClose={() => setShowAddPc(false)}
			>
				<Advanced
					port={port}
					peer={peer}
					onPort={setPort}
					onPeer={setPeer}
					disabled={running}
					manual={manual}
					onAdd={add}
					onRemove={remove}
				/>
			</Modal>

			<Modal
				title='Settings'
				open={showSettings}
				onClose={() => setShowSettings(false)}
			>
				<Diagnostics stats={stats} running={running} />
			</Modal>
		</div>
	);
};

export default App;
