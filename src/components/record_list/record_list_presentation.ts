import { createSignal, type Accessor } from "solid-js";

export type RecordListPresentationVariant<Variant extends string> = {
  /** Stable per-list identifier used to select this presentation. */
  readonly id: Variant;
  /** Visible name for the presentation switcher. */
  readonly label: string;
};

export type RecordListPresentation<Variant extends string> = {
  /** The caller reads this to choose markup for the current presentation. */
  readonly variant: Accessor<Variant>;
  /** Changes only this list's in-memory presentation. */
  readonly selectVariant: (variant: Variant) => void;
  /** The presentations this list's content supports. */
  readonly variants: ReadonlyArray<RecordListPresentationVariant<Variant>>;
};

export type RecordListPresentationOptions<Variant extends string> = {
  /** The presentations this list's content supports. */
  readonly variants: ReadonlyArray<RecordListPresentationVariant<Variant>>;
};

function requireDeclaredVariant<Variant extends string>(
  variants: ReadonlyArray<RecordListPresentationVariant<Variant>>,
  variant: Variant,
): void {
  const variantIsDeclared = variants.some(({ id }) => id === variant);
  if (!variantIsDeclared) {
    throw new Error(
      `RecordList presentation selectVariant "${variant}" must be declared in variants.`,
    );
  }
}

/**
 * Creates presentation state for one RecordList without changing its records or selection state.
 * The state is deliberately local to the caller and is never persisted as a user preference.
 */
export function createRecordListPresentation<Variant extends string>(
  options: RecordListPresentationOptions<Variant>,
): RecordListPresentation<Variant> {
  const firstVariant = options.variants[0];
  if (firstVariant === undefined)
    throw new Error("RecordList presentation requires at least one declared variant.");

  const [variant, setVariant] = createSignal(firstVariant.id);

  function selectVariant(nextVariant: Variant): void {
    requireDeclaredVariant(options.variants, nextVariant);
    setVariant(() => nextVariant);
  }

  return { variant, selectVariant, variants: options.variants };
}

export type RecordListPresentationSwitcher<Variant extends string> = {
  /** Identifies this group of presentation controls for assistive technology. */
  readonly ariaLabel: string;
  /** The available controls, supplied unchanged from the local presentation. */
  readonly variants: ReadonlyArray<RecordListPresentationVariant<Variant>>;
  /** The caller reads this to mark the active presentation control. */
  readonly selectedVariant: Accessor<Variant>;
  /** The caller attaches this to each presentation control. */
  readonly selectVariant: (variant: Variant) => void;
};

/**
 * Returns the data a caller needs to render presentation controls, or no switcher for one variant.
 * The caller owns the surrounding markup, while this helper keeps that markup local to each list.
 */
export function recordListPresentationSwitcher<Variant extends string>(
  presentation: RecordListPresentation<Variant>,
  ariaLabel: string = "List presentation",
): RecordListPresentationSwitcher<Variant> | undefined {
  if (presentation.variants.length < 2) return undefined;

  return {
    ariaLabel,
    variants: presentation.variants,
    selectedVariant: presentation.variant,
    selectVariant: presentation.selectVariant,
  };
}
