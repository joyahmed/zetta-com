export const Alert = ({ message }: AlertProps) => (
	<p
		role='alert'
		className='rounded-lg border border-danger bg-danger-soft px-3 py-2 text-sm text-danger'
	>
		{message}
	</p>
);
