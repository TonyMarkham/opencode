namespace OpenCode.Services;

/// <summary>
/// Constants for Tauri command names.
/// Ensures type-safety and prevents typos when invoking Tauri commands.
/// </summary>
internal static class TauriCommands
{
  /// <summary>
  /// Command to discover a running OpenCode server.
  /// </summary>
  public const string DiscoverServer = "discover_server";

  /// <summary>
  /// Command to spawn a new OpenCode server.
  /// </summary>
  public const string SpawnServer = "spawn_server";

  /// <summary>
  /// Command to check server health.
  /// </summary>
  public const string CheckHealth = "check_health";

  /// <summary>
  /// Command to stop the server.
  /// </summary>
  public const string StopServer = "stop_server";
}
