/// Which microphone and which speakers.
///
/// Worth choosing rather than always taking whatever Windows calls the default.
/// That default is regularly a webcam microphone, or a virtual device left
/// behind by a meeting app — and the failure is completely silent: the stream
/// opens, the counters move, nobody hears a thing. There was no way to tell
/// from inside the app which device was even in use.
const Picker = ({
	label,
	value,
	options,
	onChange,
	empty
}: {
	label: string;
	value: string;
	options: string[];
	onChange: (v: string) => void;
	empty: string;
}) => (
	<label className='flex min-w-0 flex-col gap-1'>
		<span className='text-xs font-medium tracking-wide text-muted uppercase'>
			{label}
		</span>
		<select
			value={value}
			onChange={e => onChange(e.currentTarget.value)}
			className='min-w-0 rounded-lg border border-line bg-surface px-3 py-2 text-sm text-ink outline-none focus:border-accent'
		>
			{/* Explicit rather than implied by an empty list: "System default"
			    is a choice somebody can come back to, and without a row for it
			    there is no way to undo picking a device. */}
			<option value=''>System default</option>
			{options.map(o => (
				<option key={o} value={o}>
					{o}
				</option>
			))}
		</select>
		{options.length === 0 && (
			<span className='text-xs text-danger'>{empty}</span>
		)}
	</label>
);

export const Devices = ({
	inputs,
	outputs,
	input,
	output,
	onChoose,
	onRefresh
}: DevicesProps) => (
	<div className='flex flex-col gap-3'>
		<Picker
			{...{
				label: 'Microphone',
				value: input,
				options: inputs,
				onChange: (v: string) => onChoose({ input: v }),
				empty: 'No microphone found. You can still hear everyone.'
			}}
		/>
		<Picker
			{...{
				label: 'Speakers',
				value: output,
				options: outputs,
				onChange: (v: string) => onChoose({ output: v }),
				empty: 'No output device found. You can still be heard.'
			}}
		/>
		<div className='flex items-center gap-3'>
			<button
				type='button'
				onClick={onRefresh}
				className='rounded-lg border border-line px-3 py-1.5 text-xs text-muted transition hover:border-faint hover:text-ink'
			>
				Rescan
			</button>
			{/* Changing either rebuilds the audio pipeline, which is a real
			    interruption — worth saying before somebody does it mid-call. */}
			<p className='text-xs text-faint'>
				Changing a device restarts audio for a moment.
			</p>
		</div>
	</div>
);
