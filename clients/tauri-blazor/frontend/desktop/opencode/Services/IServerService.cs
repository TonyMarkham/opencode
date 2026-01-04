namespace OpenCode.Services;

using Opencode.Server;

/// <summary>
/// Service for managing OpenCode server lifecycle via Tauri commands.
/// </summary>
public interface IServerService
{
  /// <summary>
  /// Discovers a running OpenCode server on localhost.
  /// </summary>
  /// <returns>ServerInfo if found, null otherwise.</returns>
  Task<ServerInfo?> DiscoverServerAsync();

  /// <summary>
  /// Spawns a new OpenCode server and waits for health check.
  /// </summary>
  /// <returns>ServerInfo of the spawned server.</returns>
  Task<ServerInfo> SpawnServerAsync();

  /// <summary>
  /// Checks if the currently connected server is healthy.
  /// </summary>
  /// <returns>True if healthy, false otherwise.</returns>
  Task<bool> CheckHealthAsync();

  /// <summary>
  /// Stops the currently connected server.
  /// </summary>
  Task StopServerAsync();
}
