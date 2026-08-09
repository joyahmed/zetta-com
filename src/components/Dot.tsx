export const Dot = ({ on }: DotProps) => (
	<span
		className={`size-2 shrink-0 rounded-full ${
			on ? 'bg-teal-500' : 'bg-slate-300 dark:bg-slate-700'
		}`}
	/>
);
