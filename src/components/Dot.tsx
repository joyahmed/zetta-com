export const Dot = ({ on }: DotProps) => (
	<span
		// Offline is `faint`, not `line`. A border colour on a panel of the same
		// family is invisible, and "no dot" and "a dot I cannot see" mean very
		// different things in a roster whose whole job is reporting absence.
		className={`size-2 shrink-0 rounded-full ${on ? 'bg-accent' : 'bg-faint'}`}
	/>
);
