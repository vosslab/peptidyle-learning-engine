// Keep native controls aligned with committed props while a change is guarded asynchronously.

export function selectChange(
  control: HTMLSelectElement,
  committed: string,
  onChange: (requested: string) => void,
): void {
  const requested = control.value;
  control.value = committed;
  onChange(requested);
}

export function inputChange(
  control: HTMLInputElement,
  committed: string,
  onChange: (requested: string) => void,
): void {
  const requested = control.value;
  control.value = committed;
  onChange(requested);
}

export function checkboxChange(
  control: HTMLInputElement,
  committed: boolean,
  onChange: (requested: boolean) => void,
): void {
  const requested = control.checked;
  control.checked = committed;
  onChange(requested);
}
