---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact/src/
repository: https://github.com/phoboslab/high_impact
explored_at: 2026-03-20
language: C
parent: exploration.md
---

# High Impact Input System - Deep Dive

**Source Files:** `input.h`, `input.c`

---

## Table of Contents

1. [Input Architecture Overview](#1-input-architecture-overview)
2. [Button/Key Enum](#2-buttonkey-enum)
3. [Action Binding System](#3-action-binding-system)
4. [Input State Management](#4-input-state-management)
5. [Analog vs. Digital Input](#5-analog-vs-digital-input)
6. [Gamepad Support](#6-gamepad-support)
7. [Mouse Handling](#7-mouse-handling)
8. [Text Input (Capture Mode)](#8-text-input-capture-mode)
9. [Platform Integration](#9-platform-integration)

---

## 1. Input Architecture Overview

### 1.1 Design Philosophy

The input system abstracts **multiple input devices** into a unified **action-based** system:

```
┌────────────────────────────────────────────────────────────┐
│                    INPUT SYSTEM                             │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │ Keyboard │  │ Gamepad  │  │  Mouse   │                 │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                 │
│       │             │             │                        │
│       └─────────────┼─────────────┘                        │
│                     │                                      │
│                     ▼                                      │
│       ┌──────────────────────────┐                        │
│       │   Button Enum (140+)     │                        │
│       │   INPUT_KEY_*, etc.      │                        │
│       └────────────┬─────────────┘                        │
│                    │                                       │
│                    ▼                                       │
│       ┌──────────────────────────┐                        │
│       │   Action Binding Table   │                        │
│       │   button -> action[]     │                        │
│       └────────────┬─────────────┘                        │
│                    │                                       │
│                    ▼                                       │
│       ┌──────────────────────────┐                        │
│       │   Action State Array     │                        │
│       │   state[ACTION_MAX]      │                        │
│       └────────────┬─────────────┘                        │
│                    │                                       │
│                    ▼                                       │
│              Game Code                                     │
│         input_state(ACTION)                                │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### 1.2 Key Concepts

| Concept | Description |
|---------|-------------|
| **Button** | Physical input (key, gamepad button, mouse button) |
| **Action** | Logical game action (jump, shoot, move_left) |
| **Binding** | Mapping from button to action |
| **State** | Current value of an action (0-1 for analog) |

---

## 2. Button/Key Enum

### 2.1 Complete Enum

```c
typedef enum {
    INPUT_INVALID = 0,

    // Keyboard (USB HID usage codes)
    INPUT_KEY_A = 4,
    INPUT_KEY_B = 5,
    INPUT_KEY_C = 6,
    INPUT_KEY_D = 7,
    INPUT_KEY_E = 8,
    INPUT_KEY_F = 9,
    INPUT_KEY_G = 10,
    INPUT_KEY_H = 11,
    INPUT_KEY_I = 12,
    INPUT_KEY_J = 13,
    INPUT_KEY_K = 14,
    INPUT_KEY_L = 15,
    INPUT_KEY_M = 16,
    INPUT_KEY_N = 17,
    INPUT_KEY_O = 18,
    INPUT_KEY_P = 19,
    INPUT_KEY_Q = 20,
    INPUT_KEY_R = 21,
    INPUT_KEY_S = 22,
    INPUT_KEY_T = 23,
    INPUT_KEY_U = 24,
    INPUT_KEY_V = 25,
    INPUT_KEY_W = 26,
    INPUT_KEY_X = 27,
    INPUT_KEY_Y = 28,
    INPUT_KEY_Z = 29,

    INPUT_KEY_1 = 30,
    INPUT_KEY_2 = 31,
    // ... 0-9
    INPUT_KEY_0 = 39,

    INPUT_KEY_RETURN = 40,
    INPUT_KEY_ESCAPE = 41,
    INPUT_KEY_BACKSPACE = 42,
    INPUT_KEY_TAB = 43,
    INPUT_KEY_SPACE = 44,

    INPUT_KEY_MINUS = 45,
    INPUT_KEY_EQUALS = 46,
    INPUT_KEY_LEFTBRACKET = 47,
    INPUT_KEY_RIGHTBRACKET = 48,
    INPUT_KEY_BACKSLASH = 49,

    // ... punctuation

    INPUT_KEY_F1 = 58,
    // ... F1-F12

    INPUT_KEY_PRINTSCREEN = 70,
    INPUT_KEY_SCROLLLOCK = 71,
    INPUT_KEY_PAUSE = 72,

    INPUT_KEY_INSERT = 73,
    INPUT_KEY_HOME = 74,
    INPUT_KEY_PAGEUP = 75,
    INPUT_KEY_DELETE = 76,
    INPUT_KEY_END = 77,
    INPUT_KEY_PAGEDOWN = 78,

    INPUT_KEY_RIGHT = 79,
    INPUT_KEY_LEFT = 80,
    INPUT_KEY_DOWN = 81,
    INPUT_KEY_UP = 82,

    INPUT_KEY_NUMLOCK = 83,
    INPUT_KEY_KP_DIVIDE = 84,
    INPUT_KEY_KP_MULTIPLY = 85,
    INPUT_KEY_KP_MINUS = 86,
    INPUT_KEY_KP_PLUS = 87,
    INPUT_KEY_KP_ENTER = 88,
    // ... KP_0-9

    // Modifiers
    INPUT_KEY_L_CTRL = 100,
    INPUT_KEY_L_SHIFT = 101,
    INPUT_KEY_L_ALT = 102,
    INPUT_KEY_L_GUI = 103,  // Windows/Command
    INPUT_KEY_R_CTRL = 104,
    INPUT_KEY_R_SHIFT = 105,
    INPUT_KEY_R_ALT = 106,

    INPUT_KEY_MAX = 107,

    // Gamepad buttons
    INPUT_GAMEPAD_A = 108,       // Bottom button
    INPUT_GAMEPAD_Y = 109,       // Right button
    INPUT_GAMEPAD_B = 110,       // Left button
    INPUT_GAMEPAD_X = 111,       // Top button

    INPUT_GAMEPAD_L_SHOULDER = 112,
    INPUT_GAMEPAD_R_SHOULDER = 113,
    INPUT_GAMEPAD_L_TRIGGER = 114,
    INPUT_GAMEPAD_R_TRIGGER = 115,

    INPUT_GAMEPAD_SELECT = 116,
    INPUT_GAMEPAD_START = 117,

    INPUT_GAMEPAD_L_STICK_PRESS = 118,
    INPUT_GAMEPAD_R_STICK_PRESS = 119,

    INPUT_GAMEPAD_DPAD_UP = 120,
    INPUT_GAMEPAD_DPAD_DOWN = 121,
    INPUT_GAMEPAD_DPAD_LEFT = 122,
    INPUT_GAMEPAD_DPAD_RIGHT = 123,

    INPUT_GAMEPAD_HOME = 124,

    // Analog stick directions (for digital mapping)
    INPUT_GAMEPAD_L_STICK_UP = 125,
    INPUT_GAMEPAD_L_STICK_DOWN = 126,
    INPUT_GAMEPAD_L_STICK_LEFT = 127,
    INPUT_GAMEPAD_L_STICK_RIGHT = 128,
    INPUT_GAMEPAD_R_STICK_UP = 129,
    INPUT_GAMEPAD_R_STICK_DOWN = 130,
    INPUT_GAMEPAD_R_STICK_LEFT = 131,
    INPUT_GAMEPAD_R_STICK_RIGHT = 132,

    INPUT_BUTTON_MAX = 139,

    // Mouse buttons
    INPUT_MOUSE_LEFT = 134,
    INPUT_MOUSE_MIDDLE = 135,
    INPUT_MOUSE_RIGHT = 136,
    INPUT_MOUSE_WHEEL_UP = 137,
    INPUT_MOUSE_WHEEL_DOWN = 138,

} button_t;
```

### 2.2 Name Mapping

```c
const char *input_button_to_name(button_t button) {
    static const char *names[] = {
        [INPUT_KEY_SPACE] = "SPACE",
        [INPUT_KEY_RETURN] = "RETURN",
        [INPUT_KEY_A] = "A",
        // ...
        [INPUT_GAMEPAD_A] = "GAMEPAD_A",
        [INPUT_MOUSE_LEFT] = "MOUSE_LEFT",
    };
    return names[button];
}

button_t input_name_to_button(const char *name) {
    for (int i = 0; i < INPUT_BUTTON_MAX; i++) {
        if (strcmp(names[i], name) == 0) {
            return i;
        }
    }
    return INPUT_INVALID;
}
```

---

## 3. Action Binding System

### 3.1 Binding Table

```c
#define INPUT_ACTION_MAX 32
#define INPUT_ACTION_NONE 255

static uint8_t button_to_action[INPUT_BUTTON_MAX];  // Button -> Action
static float action_state[INPUT_ACTION_MAX];        // Current state
static float action_prev_state[INPUT_ACTION_MAX];   // Previous frame

void input_init(void) {
    input_unbind_all();
}

void input_bind(button_t button, uint8_t action) {
    if (button >= INPUT_BUTTON_MAX || action >= INPUT_ACTION_MAX) return;
    button_to_action[button] = action;
}

void input_unbind(button_t button) {
    if (button >= INPUT_BUTTON_MAX) return;
    button_to_action[button] = INPUT_ACTION_NONE;
}

void input_unbind_all(void) {
    memset(button_to_action, INPUT_ACTION_NONE, sizeof(button_to_action));
}
```

### 3.2 Usage Pattern

```c
// In your game init
enum {
    ACTION_JUMP,
    ACTION_LEFT,
    ACTION_RIGHT,
    ACTION_SHOOT,
    ACTION_PAUSE,
};

void player_init(void) {
    // Keyboard bindings
    input_bind(INPUT_KEY_SPACE, ACTION_JUMP);
    input_bind(INPUT_KEY_LEFT, ACTION_LEFT);
    input_bind(INPUT_KEY_RIGHT, ACTION_RIGHT);
    input_bind(INPUT_KEY_Z, ACTION_SHOOT);
    input_bind(INPUT_KEY_ESCAPE, ACTION_PAUSE);

    // Gamepad bindings (same actions!)
    input_bind(INPUT_GAMEPAD_A, ACTION_JUMP);
    input_bind(INPUT_GAMEPAD_DPAD_LEFT, ACTION_LEFT);
    input_bind(INPUT_GAMEPAD_DPAD_RIGHT, ACTION_RIGHT);
    input_bind(INPUT_GAMEPAD_X, ACTION_SHOOT);
    input_bind(INPUT_GAMEPAD_START, ACTION_PAUSE);
}
```

### 3.3 Configuration File

```c
// Load bindings from JSON
json_t *config = platform_load_userdata("config.json");
if (config) {
    json_t *bindings = config->children;
    while (bindings) {
        const char *name = bindings->key;
        const char *button_name = bindings->value.string;

        button_t button = input_name_to_button(button_name);
        uint8_t action = input_action_for_name(name);

        if (button != INPUT_INVALID) {
            input_bind(button, action);
        }

        bindings = bindings->next;
    }
    temp_free(config);
}
```

---

## 4. Input State Management

### 4.1 State Query Functions

```c
// Get current state (0-1 for analog, 0 or 1 for digital)
float input_state(uint8_t action) {
    if (action >= INPUT_ACTION_MAX) return 0;
    return action_state[action];
}

// True on the frame the button was pressed
bool input_pressed(uint8_t action) {
    if (action >= INPUT_ACTION_MAX) return false;
    return action_state[action] > INPUT_DEADZONE &&
           action_prev_state[action] <= INPUT_DEADZONE;
}

// True on the frame the button was released
bool input_released(uint8_t action) {
    if (action >= INPUT_ACTION_MAX) return false;
    return action_state[action] <= INPUT_DEADZONE &&
           action_prev_state[action] > INPUT_DEADZONE;
}
```

### 4.2 Frame Update

```c
void input_clear(void) {
    // Copy current state to previous
    memcpy(action_prev_state, action_state, sizeof(action_state));
}

void input_set_button_state(button_t button, float state) {
    if (button >= INPUT_BUTTON_MAX) return;

    uint8_t action = button_to_action[button];
    if (action != INPUT_ACTION_NONE) {
        action_state[action] = state;
    }
}
```

### 4.3 Deadzone Handling

```c
#define INPUT_DEADZONE 0.1f  // For analog sticks

void input_set_button_state(button_t button, float state) {
    // Apply deadzone
    if (fabsf(state) < INPUT_DEADZONE) {
        state = 0;
    }

    uint8_t action = button_to_action[button];
    if (action != INPUT_ACTION_NONE) {
        action_state[action] = state;
    }

    // Handle directional mapping for sticks
    if (button == INPUT_GAMEPAD_L_STICK_UP) {
        // Map to separate actions
        if (state > 0) {
            action_state[ACTION_UP] = state;
        }
    }
}
```

---

## 5. Analog vs. Digital Input

### 5.1 Digital Input

Keyboard keys and gamepad buttons are **digital** (0 or 1):

```c
// Platform callback for keyboard
void on_key(int key, bool pressed) {
    input_set_button_state(key, pressed ? 1.0f : 0.0f);
}
```

### 5.2 Analog Input

Gamepad sticks and triggers are **analog** (0 to 1):

```c
// Platform callback for gamepad axis
void on_axis(int axis, float value) {
    //value is -1.0 to 1.0

    // Apply deadzone
    if (fabsf(value) < INPUT_DEADZONE) {
        value = 0;
    }

    // Map axis to buttons
    if (axis == AXIS_LEFT_X) {
        if (value < 0) {
            input_set_button_state(INPUT_GAMEPAD_L_STICK_LEFT, -value);
            input_set_button_state(INPUT_GAMEPAD_L_STICK_RIGHT, 0);
        } else if (value > 0) {
            input_set_button_state(INPUT_GAMEPAD_L_STICK_RIGHT, value);
            input_set_button_state(INPUT_GAMEPAD_L_STICK_LEFT, 0);
        } else {
            input_set_button_state(INPUT_GAMEPAD_L_STICK_LEFT, 0);
            input_set_button_state(INPUT_GAMEPAD_L_STICK_RIGHT, 0);
        }
    }
}
```

### 5.3 Movement Example

```c
void player_update(entity_t *self) {
    // Analog movement (gamepad stick)
    float move_x = input_state(ACTION_LEFT);
    if (move_x <= INPUT_DEADZONE) {
        move_x = -input_state(ACTION_RIGHT);
    }

    // Digital fallback (keyboard)
    if (move_x == 0) {
        if (input_state(ACTION_LEFT) > INPUT_DEADZONE) move_x = -1;
        if (input_state(ACTION_RIGHT) > INPUT_DEADZONE) move_x = 1;
    }

    self->vel.x = move_x * self->max_velocity_x;

    // Analog jump (trigger pressure)
    float jump = input_state(ACTION_JUMP);
    if (input_pressed(ACTION_JUMP)) {
        if (jump > 0.5f) {
            // High jump
            self->vel.y = -self->max_velocity_y * 1.5f;
        } else {
            // Normal jump
            self->vel.y = -self->max_velocity_y;
        }
    }
}
```

---

## 6. Gamepad Support

### 6.1 Button Layout

```
        ┌─────────────┐
        │   SELECT    │  START
        └─────────────┘
              │
    ┌─────────┴─────────┐
    │  L_SHOULDER       │  R_SHOULDER
    │  L_TRIGGER        │  R_TRIGGER
    │                   │
    │    ┌─────┐        │
    │    │  Y  │        │  D-PAD
    │ ┌──┼─────┼──┐     │   ┌─┐
    │ │  │     │  │     │  ─┼─
    │ │  │     │  │     │   └─┘
    │ └──┼─────┼──┘     │
    │    │  B  │        │  L_STICK_*
    │    └─────┘        │  R_STICK_*
    │                   │
    └───────────────────┘
```

### 6.2 Button Mapping (XInput-style)

| Button | XInput | SDL | Usage |
|--------|--------|-----|-------|
| `INPUT_GAMEPAD_A` | A | SDL_GAMEPAD_BUTTON_SOUTH | Jump/Confirm |
| `INPUT_GAMEPAD_B` | B | SDL_GAMEPAD_BUTTON_EAST | Cancel/Attack |
| `INPUT_GAMEPAD_X` | X | SDL_GAMEPAD_BUTTON_WEST | Shoot |
| `INPUT_GAMEPAD_Y` | Y | SDL_GAMEPAD_BUTTON_NORTH | Special |
| `INPUT_GAMEPAD_L_SHOULDER` | LB | SDL_GAMEPAD_BUTTON_LEFT_SHOULDER | Block |
| `INPUT_GAMEPAD_R_SHOULDER` | RB | SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER | Aim |
| `INPUT_GAMEPAD_L_TRIGGER` | LT | Analog | Accelerate |
| `INPUT_GAMEPAD_R_TRIGGER` | RT | Analog | Brake |
| `INPUT_GAMEPAD_START` | Start | SDL_GAMEPAD_BUTTON_START | Pause |
| `INPUT_GAMEPAD_SELECT` | Back | SDL_GAMEPAD_BUTTON_BACK | Menu |

### 6.3 Platform Integration (SDL)

```c
// platform_sdl.c
void platform_handle_events(void) {
    SDL_Event event;
    while (SDL_PollEvent(&event)) {
        switch (event.type) {
            case SDL_CONTROLLERBUTTONDOWN:
            case SDL_CONTROLLERBUTTONUP: {
                button_t button = sdl_to_button(event.cbutton.button);
                input_set_button_state(button,
                    event.cbutton.state == SDL_PRESSED ? 1.0f : 0.0f);
                break;
            }

            case SDL_CONTROLLERAXISMOTION: {
                float value = event.caxis.value / 32767.0f;
                handle_axis(event.caxis.axis, value);
                break;
            }
        }
    }
}

button_t sdl_to_button(Uint8 sdl_button) {
    switch (sdl_button) {
        case SDL_CONTROLLER_BUTTON_A: return INPUT_GAMEPAD_A;
        case SDL_CONTROLLER_BUTTON_B: return INPUT_GAMEPAD_B;
        case SDL_CONTROLLER_BUTTON_X: return INPUT_GAMEPAD_X;
        case SDL_CONTROLLER_BUTTON_Y: return INPUT_GAMEPAD_Y;
        case SDL_CONTROLLER_BUTTON_LEFTSHOULDER: return INPUT_GAMEPAD_L_SHOULDER;
        case SDL_CONTROLLER_BUTTON_RIGHTSHOULDER: return INPUT_GAMEPAD_R_SHOULDER;
        case SDL_CONTROLLER_BUTTON_START: return INPUT_GAMEPAD_START;
        case SDL_CONTROLLER_BUTTON_BACK: return INPUT_GAMEPAD_SELECT;
        // ...
        default: return INPUT_INVALID;
    }
}
```

---

## 7. Mouse Handling

### 7.1 Mouse Position

```c
static vec2i_t mouse_pos = {0, 0};

vec2_t input_mouse_pos(void) {
    // Convert real pixels to logical
    return vec2(
        mouse_pos.x / render_state.scale.x,
        mouse_pos.y / render_state.scale.y
    );
}

// Platform callback
void input_set_mouse_pos(int32_t x, int32_t y) {
    mouse_pos = vec2i(x, y);
}
```

### 7.2 Mouse Buttons

```c
// Mouse buttons as actions
input_bind(INPUT_MOUSE_LEFT, ACTION_ATTACK);
input_bind(INPUT_MOUSE_RIGHT, ACTION_ALT_ATTACK);

// Or direct state check
void ui_update(void) {
    vec2_t pos = input_mouse_pos();
    if (input_pressed(ACTION_MOUSE_LEFT)) {
        // Check hover
        if (rect_contains(button_rect, pos)) {
            button_click();
        }
    }
}
```

### 7.3 Mouse Wheel

```c
void input_textinput(int32_t ascii_char) {
    // Also used for mouse wheel
    if (ascii_char > 0) {
        input_set_button_state(INPUT_MOUSE_WHEEL_UP, 1.0f);
    } else if (ascii_char < 0) {
        input_set_button_state(INPUT_MOUSE_WHEEL_DOWN, 1.0f);
    }
}
```

---

## 8. Text Input (Capture Mode)

### 8.1 Capture Callback

```c
typedef void(*input_capture_callback_t)(
    void *user,
    button_t button,
    int32_t ascii_char
);

static input_capture_callback_t capture_cb = NULL;
static void *capture_user = NULL;

void input_capture(input_capture_callback_t cb, void *user) {
    capture_cb = cb;
    capture_user = user;
}

// Install callback to capture all input
input_capture(text_input_callback, text_buffer);

// Uninstall callback
input_capture(NULL, NULL);
```

### 8.2 Text Input Handler

```c
void input_textinput(int32_t ascii_char) {
    // Send to capture callback first
    if (capture_cb) {
        capture_cb(capture_user, INPUT_INVALID, ascii_char);
        return;
    }

    // Normal game input processing
    // ...
}

// Example text input callback
void text_input_callback(void *user, button_t button, int32_t ascii_char) {
    char *buffer = (char *)user;

    if (ascii_char == 8) {  // Backspace
        int len = strlen(buffer);
        if (len > 0) buffer[len - 1] = 0;
    } else if (ascii_char == 13) {  // Enter
        submit_text(buffer);
    } else if (ascii_char >= 32 && ascii_char < 127) {
        int len = strlen(buffer);
        if (len < MAX_TEXT_LEN) {
            buffer[len] = ascii_char;
            buffer[len + 1] = 0;
        }
    }
}
```

### 8.3 Deadzone for Capture

```c
#define INPUT_DEADZONE_CAPTURE 0.5f

void input_set_button_state(button_t button, float state) {
    // If in capture mode, only pass significant inputs
    if (capture_cb) {
        if (state < INPUT_DEADZONE_CAPTURE) return;
    }

    // Normal processing...
}
```

---

## 9. Platform Integration

### 9.1 SDL Backend

```c
// platform_sdl.c
void input_init(void) {
    SDL_InitSubSystem(SDL_INIT_GAMECONTROLLER);

    // Open first gamepad
    if (SDL_NumJoysticks() > 0) {
        SDL_GameControllerOpen(0);
    }
}

void input_cleanup(void) {
    SDL_QuitSubSystem(SDL_INIT_GAMECONTROLLER);
}

void platform_handle_events(void) {
    SDL_Event event;
    while (SDL_PollEvent(&event)) {
        switch (event.type) {
            case SDL_KEYDOWN:
            case SDL_KEYUP:
                input_set_button_state(event.key.keysym.scancode,
                    event.key.state == SDL_PRESSED ? 1.0f : 0.0f);
                break;

            case SDL_MOUSEMOTION:
                input_set_mouse_pos(event.motion.x, event.motion.y);
                break;

            case SDL_MOUSEBUTTONDOWN:
            case SDL_MOUSEBUTTONUP:
                input_set_button_state(
                    INPUT_MOUSE_LEFT + event.button.button - 1,
                    event.button.state == SDL_PRESSED ? 1.0f : 0.0f
                );
                break;

            case SDL_CONTROLLERBUTTONDOWN:
            case SDL_CONTROLLERBUTTONUP:
                // ... (see section 6)
                break;

            case SDL_CONTROLLERAXISMOTION:
                // ... (see section 6)
                break;

            case SDL_TEXTINPUT:
                input_textinput(event.text.text[0]);
                break;
        }
    }
}
```

### 9.2 Sokol Backend

```c
// platform_sokol.c
static void input_event(const sapp_event *event) {
    switch (event->type) {
        case SAPP_EVENTTYPE_KEY_DOWN:
        case SAPP_EVENTTYPE_KEY_UP:
            input_set_button_state(event->key_code,
                event->type == SAPP_EVENTTYPE_KEY_DOWN ? 1.0f : 0.0f);
            break;

        case SAPP_EVENTTYPE_MOUSE_MOVE:
            input_set_mouse_pos(event->mouse_x, event->mouse_y);
            break;

        case SAPP_EVENTTYPE_MOUSE_DOWN:
        case SAPP_EVENTTYPE_MOUSE_UP:
            input_set_button_state(
                INPUT_MOUSE_LEFT + event->mouse_button,
                event->type == SAPP_EVENTTYPE_MOUSE_DOWN ? 1.0f : 0.0f
            );
            break;

        case SAPP_EVENTTYPE_CHAR:
            input_textinput(event->char_code);
            break;

        case SAPP_EVENTTYPE_TOUCHES_BEGAN:
            // Touch support
            break;
    }
}

sapp_desc sokol_main(int argc, char *argv[]) {
    return (sapp_desc){
        .init_cb = init,
        .frame_cb = frame,
        .event_cb = input_event,
        // ...
    };
}
```

---

## Related Documents

- **[Main Exploration](./exploration.md)** - Overall architecture
- **[Platform Layer Deep Dive](./platform-layer-deep-dive.md)** - SDL/Sokol integration
- **[Entity System Deep Dive](./entity-system-deep-dive.md)** - Using input in entities
