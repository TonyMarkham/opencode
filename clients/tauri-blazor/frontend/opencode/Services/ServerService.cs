using OpenCode.Services.Exceptions;

namespace OpenCode.Services;

using Microsoft.JSInterop;
using Opencode.Server;
using System.Text.Json;

/// <summary>
/// Service implementation for managing OpenCode server lifecycle via Tauri commands.
/// </summary>
public class ServerService : IServerService
{
  private readonly IJSRuntime _jsRuntime;

  private static readonly JsonSerializerOptions JsonOptions = new()
  {
    PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower
  };

  /// <summary>
  /// Initializes a new instance of the <see cref="ServerService"/> class.
  /// </summary>
  /// <param name="jsRuntime">JavaScript interop runtime for calling Tauri commands.</param>
  public ServerService(IJSRuntime jsRuntime)
  {
    _jsRuntime = jsRuntime;
  }

  /// <inheritdoc />
  public async Task<ServerInfo?> DiscoverServerAsync()
  {
    try
    {
      var result = await _jsRuntime.InvokeAsync<JsonElement>(
        TauriConstants.EvalMethod,
        $"{TauriConstants.InvokePrefix}('{TauriCommands.DiscoverServer}')"
      ).ConfigureAwait(false);

      return result.ValueKind == JsonValueKind.Null
        ? null
        : JsonSerializer.Deserialize<ServerInfo>(
          result.GetRawText(),
          JsonOptions);
    }
    catch (JSException ex)
    {
      throw new ServerDiscoveryException("Tauri discover_server command failed",
        ex);
    }
  }

  /// <inheritdoc />
  public async Task<ServerInfo> SpawnServerAsync()
  {
    try
    {
      var result = await _jsRuntime.InvokeAsync<JsonElement>(
        TauriConstants.EvalMethod,
        $"{TauriConstants.InvokePrefix}('{TauriCommands.SpawnServer}')"
      ).ConfigureAwait(false);

      return JsonSerializer.Deserialize<ServerInfo>(
               result.GetRawText(),
               JsonOptions)
             ?? throw new ServerSpawnException(
               "Failed to spawn server: deserialization returned null");
    }
    catch (JSException ex)
    {
      throw new ServerSpawnException("Tauri spawn_server command failed", ex);
    }
  }

  /// <inheritdoc />
  public async Task<bool> CheckHealthAsync()
  {
    try
    {
      return await _jsRuntime.InvokeAsync<bool>(
        TauriConstants.EvalMethod,
        $"{TauriConstants.InvokePrefix}('{TauriCommands.CheckHealth}')"
      ).ConfigureAwait(false);
    }
    catch (JSException ex)
    {
      throw new ServerHealthCheckException("Tauri check_health command failed",
        ex);
    }
  }

  /// <inheritdoc />
  public async Task StopServerAsync()
  {
    try
    {
      await _jsRuntime.InvokeAsync<object>(
        TauriConstants.EvalMethod,
        $"{TauriConstants.InvokePrefix}('{TauriCommands.StopServer}')"
      ).ConfigureAwait(false);
    }
    catch (JSException ex)
    {
      throw new ServerStopException("Tauri stop_server command failed", ex);
    }
  }
}
