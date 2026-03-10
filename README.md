<p align="center">
  <img src="assets/logo.png" width="200">
</p>

# Raspberry Pi Micro-Agent

### Advancement of the project

- [x] Communication with user
- [x] Task scheduling
- [x] Tool identifications
- [x] LLM calling
- [ ] Tools implementation
  - [ ] Appointement saving
  - [ ] IO checking
  - [x] Ask LLM
  - [ ] Bill saving

### Summary

Lightweight micro-agent running on a Raspberry Pi for LLM function calling and task orchestration.

User commands are queued and processed asynchronously throughout the day. We reduce the list of tool candidate using embedding and cosinus similarity. 

*Already tried on a Raspberry Pi 5 8GB of RAM and 128GB of NVMe M.2 SSD*

### Structure of the project

- [main.rs](crates/agent-core/src/main.rs) : Define the main loop of the agent.
- [agent-interface](crates/agent-interface/src/lib.rs) : Methods to communicate with the user.
- [task-scheduler](crates/task-scheduler/src/) : Methods to save, update and call tasks.
- [embedding](crates/encoder/src/) : Methods to embedd tools definitions or incoming messages.
- [LLM inference](crates/inference/src/lib.rs) : Manage download and call of the LLM.
- [reminder](crates/reminder/src/) : TBI; Will be use to manage memory of the agent.
- [function-caller](crates/function-caller/src/) : TBI; Will be use to centralized tool calling.
- [tools](crates/tools/src/) : TBI; Will be use to define all the tool we want to call.

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