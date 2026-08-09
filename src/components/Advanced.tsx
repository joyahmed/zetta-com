import { Field } from './Field';

/// The escape hatch, not the main road. Discovery supplies peers on a normal
/// network; this is for the ones that filter mDNS, and for a PC on another
/// subnet that will never be discovered at all.
export const Advanced = ({
	port,
	peer,
	onPort,
	onPeer,
	disabled
}: AdvancedProps) => (
	<div className='flex items-end gap-3'>
		<Field
			label='Port'
			value={port}
			onChange={onPort}
			disabled={disabled}
			className='w-24'
		/>
		<Field
			label='Address'
			value={peer}
			onChange={onPeer}
			disabled={disabled}
			placeholder='192.168.0.142:9001'
			className='flex-1'
		/>
	</div>
);
