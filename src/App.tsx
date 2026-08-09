import { Advanced } from './components/Advanced';
import { Alert } from './components/Alert';
import { Diagnostics } from './components/Diagnostics';
import { Disclosure } from './components/Disclosure';
import { Messages } from './components/Messages';
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

	// Named for the talk bar and the message box, so it always says who is
	// about to be addressed rather than leaving it to be remembered.
	const targetName =
		target === null
			? 'everyone'
			: (peers.find(p => p.addr === target)?.name ?? target);

	return (
		<main className='min-h-screen bg-slate-50 px-6 py-8 text-slate-900 dark:bg-slate-950 dark:text-slate-100'>
			<div className='mx-auto flex w-full max-w-lg flex-col gap-5'>
				<header className='flex items-start justify-between'>
					<div>
						<h1 className='text-xl font-semibold tracking-tight'>
							Intercom
						</h1>
						<p className='text-xs text-slate-500 dark:text-slate-400'>
							{me ? `You are ${me}` : ' '}
						</p>
					</div>
					<button
						type='button'
						onClick={running ? stop : start}
						className={`rounded-lg px-4 py-2 text-sm font-medium text-white transition ${
							running
								? 'bg-slate-700 hover:bg-slate-600'
								: 'bg-teal-600 hover:bg-teal-500'
						}`}
					>
						{running ? 'Stop' : 'Start'}
					</button>
				</header>

				{error && <Alert message={error} />}

				{running && <TalkBar held={held} key_='F8' to={targetName} />}

				<Roster
					peers={peers}
					running={running}
					target={target}
					onTarget={setTarget}
				/>

				<Presets
					presets={presets}
					onSend={send}
					disabled={!running}
				/>

				<Messages
					messages={messages}
					onSend={send}
					disabled={!running}
					to={targetName}
				/>

				<Disclosure
					label='Shortcuts'
					open={showShortcuts}
					onOpenChange={setShowShortcuts}
				>
					<Shortcuts shortcuts={shortcuts} />
				</Disclosure>

				<Disclosure
					label='Advanced'
					open={showAddPc || undefined}
					onOpenChange={setShowAddPc}
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
				</Disclosure>

				<Disclosure label='Diagnostics'>
					<Diagnostics stats={stats} running={running} />
				</Disclosure>
			</div>
		</main>
	);
};

export default App;
