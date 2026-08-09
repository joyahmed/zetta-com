export const Field = ({
	label,
	value,
	onChange,
	disabled,
	placeholder,
	className = ''
}: FieldProps) => (
	<label className={`flex flex-col gap-1 text-left ${className}`}>
		<span className='text-xs font-medium tracking-wide text-muted uppercase'>
			{label}
		</span>
		<input
			value={value}
			onChange={e => onChange(e.currentTarget.value)}
			disabled={disabled}
			placeholder={placeholder}
			className='rounded-lg border border-line bg-surface px-3 py-2 font-mono text-sm text-ink outline-none transition placeholder:text-faint focus:border-accent disabled:cursor-not-allowed disabled:opacity-55'
		/>
	</label>
);
