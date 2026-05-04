# Task Template

---

### Template Section

All must follow our markdown engineering directive, writing the markdown and building the html with ./markdown_engineering/documentation_directive.md and build.py. Each must be detailed, not light, must be deep, pull the AHA! moments and be detailed and clear so someone junior in technical expertise can understand things fully and properly. Write fundamental documentation files to help engineers level up quickly with the gaps, ideas, technical design, data structures and processes used. See examples like ./pi and ./hermes. Follow the documentation directive.

**CRITICAL: Depth is non-negotiable.** A 200-line markdown for a large, multi-file project is unacceptable. The purpose of exploration is to teach. Read every source file. Document every significant function, type, algorithm, and data structure. Include actual code snippets with file paths. Length is not a constraint — write as much as needed to fully teach the project. If a project needs 50 pages, write 50 pages. Short documents are a failure of thoroughness, not a virtue. Grandfather review is mandatory, not optional.

**Directory Structure:** Each project's exploration lives in a subdirectory named after the project itself within the parent exploration directory. For example, the datastar source at `src.datastar/datastar/` produces documentation at `./src.datastar/datastar/` with its own `spec.md`, `markdown/`, and `html/` inside. Same for orbitinghail (`./src.orbitinghail/orbitinghail/`) and ui (`./src.ui/ui/`). A central `index.html` at the parent level (`./src.datastar/html/index.html`) points to each project's documentation.

Review tasks.md and fix markdown as well before starting.


1. [ ] The [exploration-name] exploration is too light, we need to make it more detailed, going into:
1. How each part of the project works
1. How it accesses the WebGPU layer
1. How it builds that into TypeScript types and implementation
1. What it will take to replicate similar in Rust.

1. [ ] [project directory]
1. It needs to be very deep and detailed
1. It should be created in a `./[provided-parent-directory-name-above]` directory and any sub-directory should exist under this directory and nowhere else
1. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
1. Do a good cover and add fundamental explainers for the algorithms, concepts used, how the WASM web rendering works, what graphics and SVG/vector algorithms
1. How do we replicate the C++ implementation of rendering, animation, vector graphics, optimizations in the project in Rust, what we need to keep in mind
1. How do we build a resilient storage system like this for an inexperienced software engineer

1. [ ] [project directory]
1. It needs to be very deep and detailed
1. It should be created in a `./[provided-parent-directory-name-above]` directory and any sub-directory should exist under this directory and nowhere else
1. Include an additional "what this looks like in Rust" and "what a production grade version looks like"
1. Do a good cover and add fundamental explainers for the algorithms, concepts used, how this works in WASM and wasm web
1. How do we replicate the implementation in the project in Rust, what we need to keep in mind
1. How do we build a resilient system like this for an inexperienced software engineer from first principles to expert level covering all topics related to this project.

---
