namespace OpenCode.Services.Exceptions;

/// <summary>
/// Base exception for server operation failures.
/// </summary>
public abstract class ServerOperationException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerOperationException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    protected ServerOperationException(string message) : base(message) { }

    /// <summary>
    /// Initializes a new instance of the <see cref="ServerOperationException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The inner exception.</param>
    protected ServerOperationException(string message, Exception innerException) 
        : base(message, innerException) { }
}

/// <summary>
/// Exception thrown when server spawning fails.
/// </summary>
public class ServerSpawnException : ServerOperationException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerSpawnException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    public ServerSpawnException(string message) : base(message) { }

    /// <summary>
    /// Initializes a new instance of the <see cref="ServerSpawnException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The inner exception.</param>
    public ServerSpawnException(string message, Exception innerException) 
        : base(message, innerException) { }
}

/// <summary>
/// Exception thrown when server discovery fails.
/// </summary>
public class ServerDiscoveryException : ServerOperationException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerDiscoveryException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    public ServerDiscoveryException(string message) : base(message) { }

    /// <summary>
    /// Initializes a new instance of the <see cref="ServerDiscoveryException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The inner exception.</param>
    public ServerDiscoveryException(string message, Exception innerException) 
        : base(message, innerException) { }
}

/// <summary>
/// Exception thrown when server health check fails.
/// </summary>
public class ServerHealthCheckException : ServerOperationException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerHealthCheckException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    public ServerHealthCheckException(string message) : base(message) { }
    
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerHealthCheckException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The inner exception.</param>
    public ServerHealthCheckException(string message, Exception innerException) 
        : base(message, innerException) { }
}

/// <summary>
/// Exception thrown when stopping server fails.
/// </summary>
public class ServerStopException : ServerOperationException
{
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerStopException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    public ServerStopException(string message) : base(message) { }
    
    /// <summary>
    /// Initializes a new instance of the <see cref="ServerStopException"/> class.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The inner exception.</param>
    public ServerStopException(string message, Exception innerException) 
        : base(message, innerException) { }
}
