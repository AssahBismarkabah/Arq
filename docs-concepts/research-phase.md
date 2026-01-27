Arq Research Phase
==================

The Research phase is the first of three phases in Arq. It ensures developers
understand their codebase and context before planning or writing any code.

Quick Start
-----------

* Open Arq tab in your IDE
* Click [Researcher] tab
* Enter what you want to do in the chat input
* Review findings and validate understanding
* Approve to generate research document

Core Principle
--------------

"Understand before you build."

The Research phase prevents the common pattern of AI generating code faster
than developers can comprehend. It forces deliberate understanding before
any code is written.


What Happens
============

The Research Phase Flow
-----------------------

1. User enters prompt describing what they want to do
2. Arq performs deep research (progress shown via checklist)
3. Draft findings appear in chat as a document
4. User validates or corrects in the same chat
5. When approved, artifacts are generated and saved
6. User can proceed to Planner phase


Research Activities
-------------------

When you enter a prompt, Arq performs:

* Deep Codebase Analysis
    - Scans workspace structure and architecture
    - Maps dependencies between files and modules
    - Identifies relevant code paths
    - Uses tree-sitter for accurate parsing

* Web Research
    - Searches for concepts related to your task
    - Finds documentation for external libraries
    - Checks for recent changes or deprecations
    - Gathers best practices

* External Context
    - Slack integration (if configured)
    - Confluence integration (if configured)
    - Additional documentation sources
    - Git history analysis


User Interface
==============

Main View
---------

The Researcher tab contains:

* Chat input at bottom for entering prompts
* Progress checklist showing research status
* Findings document displayed inline in chat
* Canvas button to view dependency visualization
* Continue button to proceed to Planner

Progress Checklist
------------------

Shows real-time progress:

    ✓ Deep Codebase reach and mapping dependencies
    ✓ Deep Web search on concepts and changes
    ✓ Understanding additional context
    ◐ Mapping out canvas relationships
    ○ Generating research document

Findings Document
-----------------

Appears in chat after research completes:

    ┌─────────────────────────────────────────────────┐
    │ research-doc-{task}.md                          │
    │                                                 │
    │ ## Summary                                      │
    │ Your API has 12 routes in src/routes/           │
    │ No existing rate limiting found.                │
    │                                                 │
    │ ## Dependencies                                 │
    │ • Express middleware pattern in use             │
    │ • Redis available but unused                    │
    │                                                 │
    │ ## Suggested Approach                           │
    │ Middleware-based, similar to auth.ts            │
    │                                                 │
    │ [View full doc]  [View Canvas]                  │
    └─────────────────────────────────────────────────┘

User reviews this document and either:

* Types corrections in chat if something is wrong
* Approves to generate final artifacts


Canvas View
===========

Interactive Visualization
-------------------------

Click [View Canvas] to open full-screen dependency graph:

* Visual relationship map of codebase
* Shows where your changes will fit
* Interactive: pan, zoom, click nodes
* Click [Back to Researcher] to return

Canvas Features
---------------

* Pan & Zoom: Navigate large codebases
* Click Node: See file details and line numbers
* Highlight Path: Show how components connect
* Change Location: Visual indicator of where new code fits

The canvas is rendered live from relationship data, not a static image.


Validation Flow
===============

Human Checkpoint
----------------

After findings appear, Arq asks: "Is this understanding correct?"

If correct:
    User clicks [Generate & Continue]
    Arq saves artifacts to .arq/ directory
    User can proceed to Planner phase

If incorrect:
    User types correction in chat
    Example: "You missed the Redis caching layer"
    Arq updates findings
    Process repeats until correct

This is the "highest leverage moment" - catching misunderstandings
before any code is written saves significant time and prevents
AI-generated complexity.


Output Artifacts
================

Directory Structure
-------------------

    project/
    ├── src/
    ├── .arq/
    │   └── {task-name}/
    │       └── research-doc.md
    └── ...

Research Document Contents
--------------------------

The generated research-doc.md includes:

    # Research: {Task Name}

    ## Task
    Description of what user wants to accomplish

    ## Codebase Analysis
    - Structure and architecture findings
    - Existing patterns identified
    - Relevant files and modules

    ## Dependencies
    - Internal dependencies mapped
    - External libraries identified
    - Integration points

    ## External Research
    - Web findings on concepts
    - Library documentation
    - Best practices discovered

    ## Suggested Approach
    - Recommended direction
    - Patterns to follow
    - Potential concerns

    ## Sources
    - File paths with line numbers
    - URLs referenced
    - Slack/Confluence links (if applicable)

Git Tracking
------------

The .arq/ directory should be committed to git:

* Creates audit trail
* Enables team review
* Preserves institutional knowledge
* Documents decision context


Integration Points
==================

External Services
-----------------

* Slack: Pull relevant conversations and context
* Confluence: Access team documentation
* Git History: Analyze past changes and patterns
* Web: Search for current best practices

IDE Integration
---------------

* VS Code: Top-level [Arq] tab alongside [Editor]
* IntelliJ: Sidebar icon opens full workspace
* Other IDEs: Native patterns for each platform

All integrations provide the same Research experience.


Next Phase
==========

After Research is complete:

1. Artifacts saved to .arq/{task-name}/
2. [Planner] tab becomes available
3. Planner loads research document as context
4. User proceeds to architectural planning

The validated understanding from Research ensures Planning
starts from a correct foundation.


Design Philosophy
=================

Why This Approach
-----------------

* Prevents "vibe coding" - no code without understanding
* Human validates AI's comprehension
* Corrections happen before code, not after
* Creates reusable documentation
* Builds institutional knowledge

The Research phase makes "slow down to understand" feel valuable
by surfacing insights developers didn't know they needed.
