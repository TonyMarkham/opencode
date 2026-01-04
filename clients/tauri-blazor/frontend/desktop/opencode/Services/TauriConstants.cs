namespace OpenCode.Services;

/// <summary>
/// Constants for Tauri JavaScript interop invocation.
/// </summary>
internal static class TauriConstants
{
  /// <summary>
  /// JavaScript eval method name for executing dynamic code.
  /// </summary>
  internal const string EvalMethod = "eval";
    
  /// <summary>
  /// Tauri internals invoke function prefix.
  /// </summary>
  internal const string InvokePrefix = "window.__TAURI_INTERNALS__.invoke";
}
