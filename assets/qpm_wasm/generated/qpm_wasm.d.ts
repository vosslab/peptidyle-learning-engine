/* tslint:disable */
/* eslint-disable */
/**
 * A host-owned PNG with finite positive logical CSS dimensions.
 */
export interface RenderCompletion {
    id: string;
    png: Uint8Array;
    width: number;
    height: number;
}

/**
 * A stateless conversion; omitted seed is zero, omitted date is the current UTC date.
 */
export interface ConvertRequest {
    /**
     * Registered reader format name.
     */
    inputFormat: string;
    /**
     * Registered writer format name.
     */
    outputFormat: string;
    /**
     * Owned source document or extracted package input.
     */
    input: ConversionInput;
    /**
     * Whether mixed item kinds are allowed; defaults to `false`.
     */
    allowMixed?: boolean;
    /**
     * Optional maximum retained item count; omission retains all items.
     */
    limit?: number;
    /**
     * Optional logical output name; omission uses the registry default.
     */
    outputName?: string;
    /**
     * Document metadata; omitted fields use adapter defaults.
     */
    document?: DocumentOptions;
    /**
     * Deterministic selection seed; defaults to zero.
     */
    shuffleSeed?: number;
}

/**
 * Canonical RDKit drawing options serialized as a JavaScript object.
 */
export interface DrawingDetails {
    explicitMethyl: boolean;
    atoms: number[];
    bonds: number[];
    legend?: string;
    highlightColour?: [number, number, number];
}

/**
 * Capabilities, naming and media policy copied from the sole format registry.
 */
export interface FormatInfo {
    /**
     * Registered format name.
     */
    name: string;
    /**
     * Whether the format can read input.
     */
    canRead: boolean;
    /**
     * Whether the format can write output.
     */
    canWrite: boolean;
    /**
     * Item kinds the format supports.
     */
    supportedKinds: string[];
    /**
     * Format media handling policy.
     */
    mediaPolicy: string;
    /**
     * Registry default logical output name.
     */
    defaultOutputName: string;
}

/**
 * Document defaults resolved once before invoking the shared engines.
 */
export interface DocumentOptions {
    /**
     * Optional document title; omission uses `Exam`.
     */
    title?: string;
    /**
     * Optional ISO date; omission uses the adapter-resolved UTC date.
     */
    date?: string;
}

/**
 * Expected parser and request errors are values rather than JavaScript exceptions.
 */
export type ConvertResult = { status: "success"; artifact: Artifact | null; itemCount: number; warnings: Warning[] } | { status: "error"; error: Diagnostic; warnings: Warning[] };

/**
 * Expected planning failures use the same diagnostics as conversion.
 */
export type RenderPlanResult = { status: "success"; jobs: RenderJob[]; wrapper: string; itemCount: number; warnings: Warning[] } | { status: "error"; error: Diagnostic; warnings: Warning[] };

/**
 * Independent checker evidence; severity and provenance are preserved separately.
 */
export interface IntegrityFinding {
    /**
     * Stable checker finding code.
     */
    code: string;
    /**
     * Checker-assigned severity.
     */
    severity: string;
    /**
     * Checker subsystem that produced the finding.
     */
    provenance: string;
    /**
     * Logical package path involved.
     */
    path: string;
    /**
     * Human-readable finding detail.
     */
    message: string;
}

/**
 * Independent integrity input, without creating a mutable item bank.
 */
export type PackageInput = { kind: "zip"; bytes: Uint8Array } | { kind: "entries"; entries: NamedBytes[] };

/**
 * Invalid transport yields an error; malformed ZIP content yields integrity findings.
 */
export type CheckPackageResult = { status: "success"; report: IntegrityReport } | { status: "error"; error: Diagnostic };

/**
 * Inventory is derived from the shared registry, including read/write directions.
 */
export interface FormatInventory {
    /**
     * Registry-derived format capabilities.
     */
    formats: FormatInfo[];
}

/**
 * One source document and its companions, or an already extracted package.
 */
export type ConversionInput = { kind: "file"; name: string; bytes: Uint8Array; companions?: NamedBytes[] } | { kind: "entries"; name: string; entries: NamedBytes[] };

/**
 * Owned logical file; serde copies bytes and emits independent JavaScript Uint8Arrays.
 */
export interface NamedBytes {
    /**
     * Logical POSIX name validated by the adapter.
     */
    name: string;
    /**
     * Independently owned payload copied across the JavaScript boundary.
     */
    bytes: Uint8Array;
}

/**
 * Recoverable diagnostics retain source ordering across the read and write stages.
 */
export interface Warning {
    /**
     * Read or write stage that produced this warning.
     */
    stage: string;
    /**
     * Stable warning category.
     */
    category: string;
    /**
     * Human-readable warning detail.
     */
    message: string;
    /**
     * Optional related format name.
     */
    format?: string;
    /**
     * Optional related item identifier.
     */
    item?: string;
    /**
     * Optional authored source or member name.
     */
    source?: string;
}

/**
 * Source-owned chemical query metadata for generic RDKit execution.
 */
export interface PeptideQuery {
    smarts: string;
    bondAtoms: [number, number];
}

/**
 * Stable error categories with all provenance available at the boundary.
 */
export interface Diagnostic {
    /**
     * Stable error category.
     */
    category: string;
    /**
     * Human-readable error detail.
     */
    message: string;
    /**
     * Optional related format name.
     */
    format?: string;
    /**
     * Optional related item identifier.
     */
    item?: string;
    /**
     * Optional authored source or member name.
     */
    source?: string;
    /**
     * Logical input or output name, independent of a particular asset/member source.
     */
    logicalName?: string;
}

/**
 * Static molecule drawing options recovered by the shared parser.
 */
export interface CanvasSpec {
    drawingDetails: DrawingDetails;
    peptideQuery?: PeptideQuery;
    smiles: string;
    legend: string | undefined;
    explicitMethyl: boolean;
    width: number;
    height: number;
    highlightAtoms: number[];
    highlightBonds: number[];
    highlightColour: [number, number, number] | undefined;
    highlightPeptideBonds: boolean;
}

/**
 * Structural errors, advisory warnings and the number of decoded regular files.
 */
export interface IntegrityReport {
    /**
     * Structural failures found during inspection.
     */
    errors: IntegrityFinding[];
    /**
     * Advisory findings from inspection.
     */
    warnings: IntegrityFinding[];
    /**
     * Number of decoded regular files.
     */
    entryCount: number;
}

/**
 * The exact logical artifact produced by the shared writer.
 */
export type Artifact = { kind: "file"; primary: NamedBytes; companions: NamedBytes[] } | { kind: "directory"; name: string; entries: NamedBytes[] };

/**
 * The host-visible job omits private bindings to original question fields.
 */
export interface RenderJob {
    id: string;
    kind: 'table' | 'canvas';
    html?: string;
    canvasSpec?: CanvasSpec;
    contentHash: string;
    dependencies: string[];
}

/**
 * Transparent completion array for the generated wasm-bindgen ABI.
 */
export type RenderCompletions = RenderCompletion[];


/**
 * Inspects owned ZIP or entry bytes and returns invalid transport as `CheckPackageResult::Error`.
 *
 * Serialization failures can throw; malformed package content becomes checker findings.
 */
export function checkPackage(input: PackageInput): CheckPackageResult;

/**
 * Copies JavaScript input into Rust ownership and returns expected failures as result values.
 *
 * Resolves the UTC date once when the request omits it; serialization failures can throw.
 */
export function convert(request: ConvertRequest): ConvertResult;

/**
 * Finalizes PNGs against a freshly reconstructed original conversion request.
 */
export function finishConvert(request: ConvertRequest, renders: RenderCompletions): ConvertResult;

/**
 * Returns the current shared-registry format inventory or a serialization exception.
 */
export function formats(): FormatInventory;

/**
 * Plans portable render jobs from the original conversion request.
 */
export function planRenderJobs(request: ConvertRequest): RenderPlanResult;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly checkPackage: (a: any) => [number, number, number];
    readonly convert: (a: any) => [number, number, number];
    readonly finishConvert: (a: any, b: any) => [number, number, number];
    readonly formats: () => [number, number, number];
    readonly planRenderJobs: (a: any) => [number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
