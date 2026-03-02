# Raspberry Pi Micro-Agent

Lightweight micro-agent running on a Raspberry Pi for LLM function calling and task orchestration.

User commands are queued and processed asynchronously throughout the day.

### Simplified State diagram of the agent
```mermaid

stateDiagram-v2
    state if_state1 <<choice>>
    state if_state2 <<choice>>

    state Loop {
    [*] --> CheckUserInput
    CheckUserInput --> if_state1
    if_state1 --> NewEmbedJob: New msg
    if_state1 --> CheckCurrentJob : None

    NewEmbedJob --> QueueJob
    CheckCurrentJob --> if_state2
    if_state2 --> JobCompleted : Complete
    if_state2 --> [*] : Running
    JobCompleted --> QueueJob
    StartNextJob --> AcknowledgeUser
    AcknowledgeUser --> [*]
    }

    QueueJob --> StartNextJob
    StartNextJob --> ProcessJob

    state Job {
    ProcessJob --> UpdateJob
    }

    UpdateJob --> AcknowledgeUser

```