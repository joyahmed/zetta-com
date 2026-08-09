export const Alert = ({ message }: AlertProps) => (
	<p
		role='alert'
		className='rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950 dark:text-rose-300'
	>
		{message}
	</p>
);
