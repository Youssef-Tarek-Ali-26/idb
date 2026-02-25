# Tasks and Execution Model

> Source: Cerebras SDK 1.4.0 — Task Identifiers and Task Execution

## Overview

CSL organizes execution around **tasks** — procedures that are activated by hardware
rather than called directly from other code. Tasks run until completion, at which point
the hardware chooses a new task to run.

## Task Types

### 1. Data Tasks

Activated by wavelet arrival. Receive the wavelet's 32-bit payload as a parameter.

```csl
// Create data task ID from a color (WSE-2) or input queue (WSE-3)
const my_task_id: data_task_id =
    if (@is_arch("wse2")) @get_data_task_id(my_color)
    else if (@is_arch("wse3")) @get_data_task_id(my_input_queue);

// Define the task — receives wavelet data
task my_data_task(wavelet_data: u32) void {
    // Process the incoming wavelet
    result[0] = wavelet_data;
}

// You can also receive two 16-bit values from a single 32-bit wavelet
task sparse_task(data: i16, index: i16) void {
    // Lower 16 bits = data, upper 16 bits = index
    buffer[index] = data;
}

// Bind task to its ID in comptime
comptime {
    @bind_data_task(my_data_task, my_task_id);
}
```

**Key properties:**
- Associated with `data_task_id`
- On WSE-2: bound to routable colors (IDs 0-23)
- On WSE-3: bound to input queues
- Activated automatically when a wavelet of the matching color arrives
- Cannot be explicitly activated — only wavelet arrival triggers them

### 2. Local Tasks

Explicitly activated by code. Used for sequencing operations.

```csl
// Create local task ID (valid range: 8-30 on WSE-3, 0-30 on WSE-2)
const main_id: local_task_id = @get_local_task_id(8);

task main() void {
    // Do some work
    process_data();
}

comptime {
    @bind_local_task(main, main_id);
    @activate(main_id);  // Activate at startup
}
```

**Key properties:**
- Associated with `local_task_id`
- Activated explicitly via `@activate(task_id)`
- Can be activated from within other tasks
- Can be blocked/unblocked

### 3. Control Tasks

Activated by wavelet arrival (like data tasks) but created from any identifier.
Used for signaling and control flow between PEs.

```csl
const ctrl_task_id: control_task_id = @get_control_task_id(10);

task ctrl_handler() void {
    // Handle control signal
    phase += 1;
}

comptime {
    @bind_control_task(ctrl_handler, ctrl_task_id);
}
```

**Key properties:**
- Associated with `control_task_id`
- Activated via wavelet arrival (like data tasks)
- The wavelet payload is used as control information
- Used for signaling phase transitions, synchronization

## Task ID Ranges

| Type | WSE-2 Range | WSE-3 Range |
|------|-------------|-------------|
| Data task (routable) | 0-23 | 0-7 |
| Local task | 0-30 | 8-30 |
| Control task | Any valid ID | Any valid ID |
| Reserved (memcpy) | 27, 28, 30 | 27, 28, 30 |

## Task Lifecycle

```
         @activate(id)
              │
              ▼
┌──────────────────────┐
│  READY (activated,   │◄──── @unblock(id)
│  not blocked)        │
└──────────┬───────────┘
           │ task picker selects
           ▼
┌──────────────────────┐
│  RUNNING             │
│  (executing task     │
│   body)              │
└──────────┬───────────┘
           │ task returns
           ▼
┌──────────────────────┐
│  IDLE                │
│  (waiting for next   │
│   activation)        │
└──────────────────────┘
```

## Blocking and Unblocking

Tasks can be blocked to prevent execution until a condition is met.

```csl
const my_task_id: data_task_id = @get_data_task_id(my_queue);

task my_task(data: u32) void {
    // Block this task to prevent re-entry during async operation
    @block(my_task_id);

    buffer[0] = data;

    // Start async operation; unblock when complete
    @mov16(out_dsd, buf_dsd, .{ .async = true, .unblock = my_task_id });
}
```

**Key builtins:**
- `@block(task_id)` — Prevent task from being scheduled
- `@unblock(task_id)` — Allow task to be scheduled again
- `.unblock = task_id` — Auto-unblock on async operation completion

## Async Operations

Many DSD operations can run asynchronously, allowing the CE to process other tasks
while the fabric handles data movement.

```csl
// Synchronous (blocks CE until complete)
@mov16(dest_dsd, src_dsd);

// Asynchronous (CE continues, unblocks task when done)
@mov16(dest_dsd, src_dsd, .{ .async = true, .unblock = some_task_id });

// Async with activate (activates a local task on completion)
@mov16(dest_dsd, src_dsd, .{ .async = true, .activate = next_task_id });
```

## Execution Ordering

The task picker on each PE selects which activated, unblocked task to run next.
There is **no guaranteed ordering** — you must use blocking/activation to enforce
sequencing.

```csl
// Pattern: Chain tasks using activate
const step1_id: local_task_id = @get_local_task_id(8);
const step2_id: local_task_id = @get_local_task_id(9);

task step1() void {
    // Do step 1 work
    process_phase1();
    // Activate step 2
    @activate(step2_id);
}

task step2() void {
    // Do step 2 work (guaranteed after step 1)
    process_phase2();
    sys_mod.unblock_cmd_stream();
}

comptime {
    @bind_local_task(step1, step1_id);
    @bind_local_task(step2, step2_id);
    @activate(step1_id);  // Start the chain
}
```

## Task Interaction with memcpy

When using the memcpy infrastructure, the system reserves certain task IDs
and provides a command stream mechanism:

```csl
// Signal that kernel computation is complete
// This unblocks the host's cmd_stream, allowing next memcpy or launch
sys_mod.unblock_cmd_stream();
```

**Reserved by memcpy:**
- Local task IDs: 27, 28, 30
- Control task IDs: 33, 34, 35, 36, 37
- Microthread 0 (WSE-3)
- Input queue 0, output queue 0, input queue 1 (WSE-3)
