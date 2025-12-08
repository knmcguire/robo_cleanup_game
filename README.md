# Robo Cleanup Game

A tile-based cleanup simulation game built with Bevy, featuring autonomous robot navigation, battery management, and task scheduling.

![Game Screenshot](screenshot.png)

## Overview

Control a robot to clean tiles and collect balls on a 5x5 grid. The robot navigates autonomously using A* pathfinding, manages its battery level, and executes queued tasks in sequence. Watch as the robot methodically cleans the environment while monitoring its power consumption.

## Features

- **Autonomous Navigation**: Click tiles or objects to queue waypoints; the robot calculates optimal paths using A* pathfinding
- **Task Management**: Queue multiple tasks (movement, ball pickup, disposal) that execute sequentially
- **Battery System**: Battery depletes based on distance traveled (10% per unit); robot automatically returns to charging station when depleted
- **Cleanliness Tracking**: Monitor overall progress as tiles are cleaned and balls are collected
- **Interactive UI**: Click waypoint buttons to remove tasks from the queue
- **Visual Feedback**: Tiles change color when cleaned, objects highlight when clicked

## Gameplay Mechanics

### Controls

- **Left Click on Tile**: Add a movement waypoint to that location
- **Left Click on Ball**: Queue a pickup task for that ball
- **Left Click on Drop Zone**: Queue a disposal task (drops all carried balls)
- **Left Click on Waypoint Button**: Remove that task from the queue

### Game Elements

- **Green Tiles**: Start dirty (dark green), become lighter when the robot passes over them
- **Yellow Balls**: Three balls spawn on the field; click to collect them
- **Red Drop Zone**: Located at grid position (-2, -2); dispose of collected balls here
- **Blue Charging Station**: Located at grid position (2, -2); robot returns here automatically when battery is low
- **3D Battery Bar**: Floats above the robot, color-coded (green/orange/red) based on charge level

### Task Types

1. **Move To**: Robot navigates to the specified tile and waits for 2 seconds
2. **Pick Up**: Robot navigates to a ball's location and collects it (balls float above the robot)
3. **Drop Away**: Robot navigates to the drop zone and releases all carried balls

### Battery Management

- Battery starts at 100% and depletes at 10% per grid unit traveled
- When battery reaches 0%, all queued tasks are cleared and the robot returns to the charging station
- Charging takes 5 seconds to reach full capacity
- Battery bar changes color: green (>50%), orange (25-50%), red (<25%), yellow (charging)

## Build Instructions

### Prerequisites

- Rust (1.70 or later)
- Cargo (comes with Rust)

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd robo_cleanup_game
```

2. Build the project:
```bash
cargo build --release
```

The compiled binary will be located at `target/release/robo_cleanup_game.exe` (Windows) or `target/release/robo_cleanup_game` (Linux/macOS).

## Running the Game

### Development Mode

Run with cargo for faster iteration (debug mode):
```bash
cargo run
```

### Release Mode

For better performance, run the optimized build:
```bash
cargo run --release
```

Or run the compiled binary directly:
```bash
./target/release/robo_cleanup_game
```

## Project Structure

```
robo_cleanup_game/
├── src/
│   └── main.rs           # Main game logic (ECS systems, pathfinding, UI)
├── Cargo.toml            # Project dependencies
├── robot.glb             # 3D robot model asset
└── screenshot.png        # Game screenshot
```

## Technical Details

- **Engine**: Bevy 0.17.3
- **Rendering**: 3D with orthographic isometric camera view
- **Pathfinding**: A* algorithm with Manhattan distance heuristic
- **Input Handling**: Mesh picking with observer pattern for click events
- **Architecture**: Entity Component System (ECS)

### Key Components

- `Robot`: Manages position, movement path, waypoint queue, battery level, and charging state
- `Tile`: Grid cells that can be cleaned
- `Ball`: Collectible objects that float above the robot when picked up
- `DropZone`: Disposal location for collected balls
- `ChargingStation`: Battery recharge location
- `Cleanliness`: Resource tracking cleaned tiles and collected balls

## License

This project is available for educational and personal use.

## Credits

Built with the Bevy game engine. Inspired by the alien-cake-addict example from the Bevy repository.