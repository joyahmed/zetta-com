import { Stat } from './Stat';

export const Diagnostics = ({ stats, running }: DiagnosticsProps) => (
	<>
		{/* Three columns, not five. Five tiles across a 460px panel leaves each
		    one about seventy pixels, which is not enough for a label like
		    "rejected" let alone a number beside it. */}
		<div className='grid grid-cols-3 gap-2'>
			<Stat {...{ label: 'sent', value: stats?.tx }} />
			<Stat {...{ label: 'received', value: stats?.rx }} />
			<Stat {...{ label: 'last seq', value: stats?.lastSeq }} />
			<Stat {...{ label: 'lost', value: stats?.lost, warn: !!stats?.lost }} />
			<Stat {...{ label: 'rejected', value: stats?.bad, warn: !!stats?.bad }} />
		</div>
		<p className='mt-3 font-mono text-xs break-all text-muted'>
			{/* The arrow used to be followed by the fallback address. That field
			    is gone — a PC discovery cannot reach is a manual entry now — so
			    what was left was an arrow pointing at nothing. */}
			{running ? `bound 0.0.0.0:${stats?.port}` : 'not bound'}
		</p>
	</>
);
