// Shared types for `CredentialPicker.svelte` — a separate module because Svelte
// component files cannot be used as type namespaces (`CredentialPicker.Option`
// fails svelte-check), and the host editor builds the option list.
/** One row of the app-drawn credential dropdown. */
export interface CredentialOption {
  /** The value written back to the form (`'none'`, `'key:<id>'`, `'identity:<id>'`). */
  value: string;
  label: string;
  /** Section header the option renders under; `null` = the ungrouped top. */
  group: string | null;
}

/** All-selectable row paired with its flat index, for keyboard highlighting. */
export interface CredentialRow {
  o: CredentialOption;
  index: number;
}
