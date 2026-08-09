export const Field = ({
	label,
	value,
	onChange,
	disabled,
	placeholder,
	className = ''
}: FieldProps) => (
	<label className={`flex flex-col gap-1 text-left ${className}`}>
		<span className='text-xs font-medium tracking-wide text-slate-500 uppercase dark:text-slate-400'>
			{label}
		</span>
		<input
			value={value}
			onChange={e => onChange(e.currentTarget.value)}
			disabled={disabled}
			placeholder={placeholder}
			className='rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-sm text-slate-900 outline-none transition placeholder:text-slate-400 focus:border-teal-500 focus:ring-2 focus:ring-teal-500/30 disabled:cursor-not-allowed disabled:opacity-55 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100'
		/>
	</label>
);
