import init from "../generated/qti_wasm.js";
/** Initialize explicitly from release Wasm bytes in browsers, workers, or Node. */
export async function initialize(bytes) {
    await init({ module_or_path: new Uint8Array(bytes) });
}
export { formats, convert, checkPackage, planRenderJobs, finishConvert } from "../generated/qti_wasm.js";
