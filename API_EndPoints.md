# OpenCode API Endpoints

## Overview

This document provides a comprehensive overview of the API endpoints exposed by the OpenCode server for session management and other functionalities.

### Session Management Endpoints

#### 1. List All Sessions

- **Endpoint**: `GET /session`
- **Description**: Retrieves a list of all sessions.
- **Output**:
  ```json
  [
    {
      "id": "session1",
      "projectID": "project1",
      "directory": "/path/to/session1",
      "title": "New session - 2025-10-10T12:00:00.000Z",
      "time": {
        "created": 1634567890,
        "updated": 1634567890
      }
    },
    {
      "id": "session2",
      "projectID": "project2",
      "directory": "/path/to/session2",
      "title": "New session - 2025-10-10T12:05:00.000Z",
      "time": {
        "created": 1634567900,
        "updated": 1634567900
      }
    }
  ]
  ```

#### 2. Get Specific Session

- **Endpoint**: `GET /session/:id`
- **Description**: Retrieves details of a specific session by its ID.
- **Output**:
  ```json
  {
    "id": "session1",
    "projectID": "project1",
    "directory": "/path/to/session1",
    "title": "New session - 2025-10-10T12:00:00.000Z",
    "time": {
      "created": 1634567890,
      "updated": 1634567890
    },
    "summary": {
      "additions": 10,
      "deletions": 2,
      "files": 5
    }
  }
  ```

#### 3. Create a New Session

- **Endpoint**: `POST /session`
- **Description**: Creates a new session. Requires session details in the request body.
- **Input**:
  ```json
  {
    "title": "My New Session",
    "projectID": "project1",
    "directory": "/path/to/new/session"
  }
  ```
- **Output**:
  ```json
  {
    "id": "session3",
    "projectID": "project1",
    "directory": "/path/to/new/session",
    "title": "My New Session",
    "time": {
      "created": 1634568000,
      "updated": 1634568000
    }
  }
  ```

#### 4. Delete a Session

- **Endpoint**: `DELETE /session/:id`
- **Description**: Deletes a session and all its associated data.
- **Output**:
  ```json
  {
    "success": true,
    "message": "Session deleted successfully."
  }
  ```

#### 5. Get Child Sessions

- **Endpoint**: `GET /session/:id/children`
- **Description**: Retrieves child sessions of a specified session.
- **Output**:
  ```json
  [
    {
      "id": "childSession1",
      "parentID": "session1",
      "title": "Child session - 2025-10-10T12:10:00.000Z"
    }
  ]
  ```

#### 6. Get Todo List for a Session

- **Endpoint**: `GET /session/:id/todo`
- **Description**: Retrieves the todo list associated with a specific session.
- **Output**:
  ```json
  [
    {
      "task": "Complete the documentation",
      "status": "pending"
    },
    {
      "task": "Review code changes",
      "status": "completed"
    }
  ]
  ```

#### 7. Share a Session

- **Endpoint**: `POST /session/:id/share`
- **Description**: Shares a session.
- **Input**:
  ```json
  {
    "url": "https://example.com/shared/session1"
  }
  ```
- **Output**:
  ```json
  {
    "id": "session1",
    "share": {
      "url": "https://example.com/shared/session1"
    }
  }
  ```

#### 8. Unshare a Session

- **Endpoint**: `DELETE /session/:id/share`
- **Description**: Unshares a session.
- **Output**:
  ```json
  {
    "success": true,
    "message": "Session unshared successfully."
  }
  ```

#### 9. Fork a Session

- **Endpoint**: `POST /session/:id/fork`
- **Description**: Creates a child session from an existing session.
- **Input**:
  ```json
  {
    "messageID": "message1"
  }
  ```
- **Output**:
  ```json
  {
    "id": "childSession2",
    "parentID": "session1",
    "title": "Child session - 2025-10-10T12:15:00.000Z"
  }
  ```

#### 10. Revert a Session

- **Endpoint**: `POST /session/:id/revert`
- **Description**: Reverts a session to a previous state.
- **Input**:
  ```json
  {
    "messageID": "message1"
  }
  ```
- **Output**:
  ```json
  {
    "success": true,
    "message": "Session reverted successfully."
  }
  ```

#### 11. Get Messages for a Session

- **Endpoint**: `GET /session/:id/message`
- **Description**: Lists messages associated with a specific session.
- **Output**:
  ```json
  [
    {
      "id": "message1",
      "sessionID": "session1",
      "content": "This is a message.",
      "time": {
        "created": 1634568100
      }
    }
  ]
  ```

### Other Functional Endpoints

#### 12. List All PTY Sessions

- **Endpoint**: `GET /pty`
- **Description**: Lists all PTY sessions.
- **Output**:
  ```json
  [
    {
      "id": "pty1",
      "status": "active"
    }
  ]
  ```

#### 13. Create a New PTY Session

- **Endpoint**: `POST /pty`
- **Description**: Creates a new PTY session.
- **Input**:
  ```json
  {
    "name": "New PTY Session"
  }
  ```
- **Output**:
  ```json
  {
    "id": "pty2",
    "status": "created"
  }
  ```

#### 14. Get PTY Session Info

- **Endpoint**: `GET /pty/:id`
- **Description**: Retrieves information about a specific PTY session.
- **Output**:
  ```json
  {
    "id": "pty1",
    "status": "active"
  }
  ```

#### 15. Update PTY Session

- **Endpoint**: `PUT /pty/:id`
- **Description**: Updates a specific PTY session.
- **Input**:
  ```json
  {
    "status": "inactive"
  }
  ```
- **Output**:
  ```json
  {
    "id": "pty1",
    "status": "inactive"
  }
  ```

#### 16. Delete a PTY Session

- **Endpoint**: `DELETE /pty/:id`
- **Description**: Deletes a specific PTY session.
- **Output**:
  ```json
  {
    "success": true,
    "message": "PTY session deleted successfully."
  }
  ```

---

This document provides a comprehensive overview of the API endpoints exposed by the OpenCode server for session management and other functionalities. If you have any further questions or need additional details, feel free to ask!
