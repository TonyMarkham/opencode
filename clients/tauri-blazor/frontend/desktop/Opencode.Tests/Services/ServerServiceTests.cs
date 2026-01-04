using Microsoft.JSInterop;
using Moq;
using OpenCode.Services;
using OpenCode.Services.Exceptions;
using Opencode.Server;
using System.Text.Json;
using System.Threading.Tasks;
using Xunit;

namespace Opencode.Tests.Services;

/// <summary>
/// Tests for ServerService - focuses on error handling and JSON deserialization.
/// These are UNIT tests (no Blazor rendering), testing service logic in isolation.
/// </summary>
public class ServerServiceTests
{
    /// <summary>
    /// VALUE: Verifies JSException is wrapped correctly in ServerDiscoveryException.
    /// 
    /// WHY THIS MATTERS: If inner exception is lost, debugging Tauri command failures
    /// becomes impossible. The inner exception contains the actual Tauri error details.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Exception wrapping loses inner exception
    /// - Wrong exception type is thrown
    /// - Error message doesn't indicate the operation that failed
    /// </summary>
    [Fact]
    public async Task GivenJSExceptionWhenDiscoveringServerThenThrowsServerDiscoveryException()
    {
        // GIVEN: JSRuntime that throws JSException
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<JsonElement>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ThrowsAsync(new JSException("Tauri command failed"));

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN: Calling DiscoverServerAsync
        var exception = await Assert.ThrowsAsync<ServerDiscoveryException>(
            () => service.DiscoverServerAsync());

        // THEN: Should wrap JSException as inner exception
        Assert.NotNull(exception.InnerException);
        Assert.IsType<JSException>(exception.InnerException);
        Assert.Contains("Tauri discover_server command failed", exception.Message);
    }

    /// <summary>
    /// VALUE: Verifies snake_case JSON deserialization works correctly.
    /// 
    /// WHY THIS MATTERS: Rust serde outputs snake_case JSON, but C# expects PascalCase by default.
    /// If deserialization breaks, all server data would be corrupted or null.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - JsonNamingPolicy is removed or changed
    /// - Protobuf field names change without updating deserializer
    /// - Property name mapping breaks
    /// </summary>
    [Fact]
    public async Task GivenValidSnakeCaseJsonWhenDiscoveringServerThenDeserializesCorrectly()
    {
        // GIVEN: JSRuntime returns snake_case JSON (matches Rust serde output)
        var json = """
        {
            "pid": 12345,
            "port": 3000,
            "base_url": "http://localhost:3000",
            "name": "opencode",
            "command": "opencode serve",
            "owned": true
        }
        """;

        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<JsonElement>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ReturnsAsync(JsonDocument.Parse(json).RootElement);

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN: Calling DiscoverServerAsync
        var result = await service.DiscoverServerAsync();

        // THEN: Should deserialize all fields correctly
        Assert.NotNull(result);
        Assert.Equal(12345u, result.Pid);
        Assert.Equal(3000u, result.Port);
        Assert.Equal("http://localhost:3000", result.BaseUrl);
        Assert.Equal("opencode", result.Name);
        Assert.Equal("opencode serve", result.Command);
        Assert.True(result.Owned);
    }

    /// <summary>
    /// VALUE: Verifies null result handling in DiscoverServerAsync.
    /// 
    /// WHY THIS MATTERS: Discovery returns null when no server is found (valid scenario).
    /// If service crashes on null instead of returning null gracefully, the UI will break.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Null handling is removed
    /// - NullReferenceException occurs on null JSON
    /// - Service doesn't distinguish between "no server" and "error"
    /// </summary>
    [Fact]
    public async Task GivenNullResultWhenDiscoveringServerThenReturnsNull()
    {
        // GIVEN: JSRuntime returns JSON null (no server found)
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<JsonElement>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ReturnsAsync(JsonDocument.Parse("null").RootElement);

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN: Calling DiscoverServerAsync
        var result = await service.DiscoverServerAsync();

        // THEN: Should return null (no server found)
        Assert.Null(result);
    }

    /// <summary>
    /// VALUE: Verifies SpawnServerAsync throws when deserialization returns null.
    /// 
    /// WHY THIS MATTERS: Unlike discovery, spawn should ALWAYS return a server.
    /// Null from spawn indicates a serious error that needs to be caught early.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Null check is removed
    /// - Service allows null ServerInfo to propagate to UI
    /// - Spawn succeeds but returns invalid data
    /// </summary>
    [Fact]
    public async Task GivenNullResultWhenSpawningServerThenThrowsServerSpawnException()
    {
        // GIVEN: JSRuntime returns JSON that deserializes to null
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<JsonElement>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ReturnsAsync(JsonDocument.Parse("null").RootElement);

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN/THEN: Should throw ServerSpawnException
        var exception = await Assert.ThrowsAsync<ServerSpawnException>(
            () => service.SpawnServerAsync());
        
        Assert.Contains("deserialization returned null", exception.Message);
    }

    /// <summary>
    /// VALUE: Verifies CheckHealthAsync returns boolean correctly for both true and false.
    /// 
    /// WHY THIS MATTERS: Health check is critical for UI state management.
    /// Wrong boolean interpretation could show "healthy" when server is down, or vice versa.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Boolean deserialization breaks
    /// - Type coercion issues occur
    /// - True/false values are inverted
    /// </summary>
    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task GivenHealthStatusWhenCheckingHealthThenReturnsCorrectBoolean(bool healthStatus)
    {
        // GIVEN: JSRuntime returns health status
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<bool>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ReturnsAsync(healthStatus);

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN: Calling CheckHealthAsync
        var result = await service.CheckHealthAsync();

        // THEN: Should return correct status
        Assert.Equal(healthStatus, result);
    }

    /// <summary>
    /// VALUE: Verifies JSException is wrapped in ServerSpawnException.
    /// 
    /// WHY THIS MATTERS: Spawn failures need specific exception type for UI error handling.
    /// Generic exceptions would make error categorization impossible.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Exception wrapping is removed
    /// - Wrong exception type is thrown
    /// - Inner exception is lost
    /// </summary>
    [Fact]
    public async Task GivenJSExceptionWhenSpawningServerThenThrowsServerSpawnException()
    {
        // GIVEN: JSRuntime throws JSException
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<JsonElement>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ThrowsAsync(new JSException("Spawn failed"));

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN/THEN: Should throw ServerSpawnException with inner JSException
        var exception = await Assert.ThrowsAsync<ServerSpawnException>(
            () => service.SpawnServerAsync());
        
        Assert.IsType<JSException>(exception.InnerException);
        Assert.Contains("Tauri spawn_server command failed", exception.Message);
    }

    /// <summary>
    /// VALUE: Verifies JSException is wrapped in ServerHealthCheckException.
    /// 
    /// WHY THIS MATTERS: Health check failures need specific exception type.
    /// This allows UI to distinguish between "server unreachable" vs other errors.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Exception wrapping is removed
    /// - Wrong exception type is thrown
    /// - Inner exception is lost
    /// </summary>
    [Fact]
    public async Task GivenJSExceptionWhenCheckingHealthThenThrowsServerHealthCheckException()
    {
        // GIVEN: JSRuntime throws JSException
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<bool>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ThrowsAsync(new JSException("Health check failed"));

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN/THEN: Should throw ServerHealthCheckException with inner JSException
        var exception = await Assert.ThrowsAsync<ServerHealthCheckException>(
            () => service.CheckHealthAsync());
        
        Assert.IsType<JSException>(exception.InnerException);
        Assert.Contains("Tauri check_health command failed", exception.Message);
    }

    /// <summary>
    /// VALUE: Verifies JSException is wrapped in ServerStopException.
    /// 
    /// WHY THIS MATTERS: Stop failures need specific exception type for error handling.
    /// Allows UI to show appropriate error messages for stop failures.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Exception wrapping is removed
    /// - Wrong exception type is thrown
    /// - Inner exception is lost
    /// </summary>
    [Fact]
    public async Task GivenJSExceptionWhenStoppingServerThenThrowsServerStopException()
    {
        // GIVEN: JSRuntime throws JSException
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<object>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ThrowsAsync(new JSException("Stop failed"));

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN/THEN: Should throw ServerStopException with inner JSException
        var exception = await Assert.ThrowsAsync<ServerStopException>(
            () => service.StopServerAsync());
        
        Assert.IsType<JSException>(exception.InnerException);
        Assert.Contains("Tauri stop_server command failed", exception.Message);
    }

    /// <summary>
    /// VALUE: Verifies SpawnServerAsync succeeds with valid data.
    /// 
    /// WHY THIS MATTERS: This is the happy path - ensures valid spawns work correctly.
    /// Regression here would break all server spawning.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Deserialization breaks for valid data
    /// - Service logic is broken by refactoring
    /// - Field mapping is incorrect
    /// </summary>
    [Fact]
    public async Task GivenValidServerDataWhenSpawningThenReturnsServerInfo()
    {
        // GIVEN: JSRuntime returns valid server data
        var json = """
        {
            "pid": 99999,
            "port": 3001,
            "base_url": "http://localhost:3001",
            "name": "opencode",
            "command": "opencode serve",
            "owned": true
        }
        """;

        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<JsonElement>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ReturnsAsync(JsonDocument.Parse(json).RootElement);

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN: Calling SpawnServerAsync
        var result = await service.SpawnServerAsync();

        // THEN: Should return populated ServerInfo
        Assert.NotNull(result);
        Assert.Equal(99999u, result.Pid);
        Assert.Equal(3001u, result.Port);
        Assert.True(result.Owned);
    }

    /// <summary>
    /// VALUE: Verifies StopServerAsync completes without error.
    /// 
    /// WHY THIS MATTERS: Stop should succeed silently (void return).
    /// Exceptions should only be thrown on actual failures.
    /// 
    /// BUG THIS CATCHES: Would catch if:
    /// - Stop throws exception on success
    /// - Service doesn't await properly
    /// - Return type handling breaks
    /// </summary>
    [Fact]
    public async Task GivenSuccessfulStopWhenStoppingServerThenCompletesWithoutError()
    {
        // GIVEN: JSRuntime completes successfully
        var mockJsRuntime = new Mock<IJSRuntime>();
        mockJsRuntime
            .Setup(x => x.InvokeAsync<object>(
                It.IsAny<string>(),
                It.IsAny<object[]>()))
            .ReturnsAsync((object?)null);

        var service = new ServerService(mockJsRuntime.Object);

        // WHEN: Calling StopServerAsync
        // THEN: Should complete without throwing
        await service.StopServerAsync();
        
        // If we get here, test passed
        Assert.True(true);
    }
}
