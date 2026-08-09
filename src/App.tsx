import { Advanced } from './components/Advanced';
import { Alert } from './components/Alert';
import { Diagnostics } from './components/Diagnostics';
import { Disclosure } from './components/Disclosure';
import { Messages } from './components/Messages';
import { Roster } from './components/Roster';
import { TalkBar } from './components/TalkBar';
import { useLocalName } from './hooks/useLocalName';
import { useManualPeers } from './hooks/useManualPeers';
import { useMessages } from './hooks/useMessages';
import { usePtt } from './hooks/usePtt';
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
	const { manual, add, remove } = useManualPeers(setError);

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

				{running && <TalkBar held={held} key_='F8' />}

				<Roster
					peers={peers}
					running={running}
					selected={peer}
					onSelect={setPeer}
				/>

				<Messages
					messages={messages}
					onSend={send}
					disabled={!running}
				/>

				<Disclosure label='Advanced'>
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
