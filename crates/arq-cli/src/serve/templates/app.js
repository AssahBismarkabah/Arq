/**
 * Arq Knowledge Graph Visualization
 *
 * Uses Sigma.js for WebGL rendering and Graphology for graph data structure.
 * ForceAtlas2 algorithm handles automatic node positioning.
 */

// =============================================================================
// Global State
// =============================================================================

let graph = null;
let renderer = null;
let fa2Layout = null;
let layoutRunning = true;
let selectedNode = null;
let hoveredNode = null;
let searchQuery = '';
let visibleTypes = new Set(); // Populated dynamically from graph data
let nodeTypeColors = new Map(); // Maps category -> color

// =============================================================================
// Initialization
// =============================================================================

/**
 * Initialize the graph visualization.
 */
async function init() {
    try {
        // Fetch graph data from API
        const response = await fetch('/api/graph');
        const data = await response.json();

        console.log('Loaded graph data:', data.nodes.length, 'nodes,', data.edges.length, 'edges');

        // Create Graphology graph
        graph = new graphology.Graph();

        // Collect node types and colors from data
        data.nodes.forEach(node => {
            const category = node.attributes.category;
            const color = node.attributes.color;
            if (category && !nodeTypeColors.has(category)) {
                nodeTypeColors.set(category, color);
                visibleTypes.add(category);
            }
        });

        // Build the filter UI based on discovered types
        buildFilterUI();

        // Import nodes with random initial positions
        data.nodes.forEach(node => {
            graph.addNode(node.key, {
                ...node.attributes,
                x: Math.random() * 100,
                y: Math.random() * 100
            });
        });

        // Import edges (skip duplicates)
        data.edges.forEach(edge => {
            if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
                try {
                    graph.addEdge(edge.source, edge.target, edge.attributes || {});
                } catch (e) {
                    // Skip duplicate edges
                }
            }
        });

        // Update stats display
        document.getElementById('node-count').textContent = graph.order;
        document.getElementById('edge-count').textContent = graph.size;

        // Hide loading indicator
        document.getElementById('loading').classList.add('hidden');

        // Handle empty graph
        if (graph.order === 0) {
            showEmptyState();
            return;
        }

        // Create Sigma renderer
        createRenderer();

        // Start ForceAtlas2 layout
        startLayout();

        // Setup event handlers
        setupEventHandlers();

    } catch (error) {
        console.error('Failed to load graph:', error);
        showError(error.message);
    }
}

/**
 * Show empty state message.
 */
function showEmptyState() {
    const loading = document.getElementById('loading');
    loading.classList.remove('hidden');
    // Clear existing content and add new elements safely
    loading.textContent = '';
    const msg = document.createElement('div');
    msg.style.color = 'var(--text-secondary)';
    msg.textContent = 'No data indexed yet. Run ';
    const code = document.createElement('code');
    code.textContent = 'arq index';
    msg.appendChild(code);
    msg.appendChild(document.createTextNode(' first.'));
    loading.appendChild(msg);
}

/**
 * Show error message.
 */
function showError(message) {
    const loading = document.getElementById('loading');
    loading.textContent = '';
    const msg = document.createElement('div');
    msg.style.color = '#cf222e';
    msg.textContent = 'Failed to load graph: ' + message;
    loading.appendChild(msg);
}

// =============================================================================
// Dynamic Filter UI
// =============================================================================

/**
 * Build filter checkboxes based on discovered node types.
 */
function buildFilterUI() {
    const container = document.getElementById('node-type-filters');

    // Clear existing filters (except the header)
    const header = container.querySelector('h3');
    container.textContent = '';
    container.appendChild(header);

    // Sort types alphabetically for consistent display
    const sortedTypes = Array.from(nodeTypeColors.entries()).sort((a, b) =>
        a[0].localeCompare(b[0])
    );

    // Create a checkbox for each type
    sortedTypes.forEach(([category, color]) => {
        const label = document.createElement('label');
        label.className = 'filter-item';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.dataset.type = category;
        checkbox.checked = true;
        checkbox.addEventListener('change', (e) => {
            if (e.target.checked) {
                visibleTypes.add(category);
            } else {
                visibleTypes.delete(category);
            }
            if (renderer) renderer.refresh();
        });

        const dot = document.createElement('span');
        dot.className = 'dot';
        dot.style.background = color;

        const text = document.createElement('span');
        // Capitalize first letter for display
        text.textContent = category.charAt(0).toUpperCase() + category.slice(1);

        label.appendChild(checkbox);
        label.appendChild(dot);
        label.appendChild(text);
        container.appendChild(label);
    });
}

// =============================================================================
// Renderer Setup
// =============================================================================

/**
 * Create and configure the Sigma renderer.
 */
function createRenderer() {
    const container = document.getElementById('graph-container');

    renderer = new Sigma(graph, container, {
        minCameraRatio: 0.05,
        maxCameraRatio: 10,
        labelRenderedSizeThreshold: 12,  // Only show labels for larger nodes
        labelDensity: 0.02,              // Show fewer labels
        labelGridCellSize: 120,          // More spacing between labels
        renderEdgeLabels: false,
        defaultNodeColor: '#0969da',
        defaultEdgeColor: '#999999',     // Darker edge color
        labelColor: { color: '#24292f' },
        labelFont: 'Arial',
        labelSize: 12,
        nodeReducer: nodeReducer,
        edgeReducer: edgeReducer
    });
}

/**
 * Start the ForceAtlas2 layout algorithm.
 */
function startLayout() {
    // forceAtlas2 and FA2Layout are imported via ES modules in index.html
    const settings = forceAtlas2.inferSettings(graph);

    fa2Layout = new FA2Layout(graph, {
        settings: {
            ...settings,
            gravity: 0.05,
            scalingRatio: 10,
            slowDown: 5
        }
    });

    fa2Layout.start();

    // Auto-stop layout after settling (for large graphs)
    setTimeout(() => {
        if (layoutRunning && graph.order > 100) {
            setTimeout(() => stopLayout(), 3000);
        }
    }, 5000);
}

// =============================================================================
// Reducers (Node/Edge Filtering & Highlighting)
// =============================================================================

/**
 * Node reducer for filtering and highlighting.
 * Called by Sigma for each node during render.
 */
function nodeReducer(node, data) {
    const res = { ...data };

    // Hide filtered types
    if (!visibleTypes.has(data.category)) {
        res.hidden = true;
        return res;
    }

    // Search filtering - dim non-matching nodes
    if (searchQuery && !data.label.toLowerCase().includes(searchQuery.toLowerCase())) {
        res.color = '#e1e4e8';
        res.label = '';
    }

    // Hover highlighting
    if (hoveredNode) {
        if (node === hoveredNode || graph.hasEdge(node, hoveredNode) || graph.hasEdge(hoveredNode, node)) {
            res.highlighted = true;
        } else {
            res.color = '#e1e4e8';
            res.label = '';
        }
    }

    // Selected node emphasis
    if (node === selectedNode) {
        res.highlighted = true;
        res.color = '#24292f';
    }

    return res;
}

/**
 * Edge reducer for highlighting.
 * Called by Sigma for each edge during render.
 */
function edgeReducer(edge, data) {
    const res = { ...data };

    // Highlight edges connected to hovered node
    if (hoveredNode) {
        const source = graph.source(edge);
        const target = graph.target(edge);

        if (source === hoveredNode || target === hoveredNode) {
            res.color = '#0969da';
            res.size = 2;
        } else {
            res.hidden = true;
        }
    }

    // Highlight edges connected to selected node
    if (selectedNode) {
        const source = graph.source(edge);
        const target = graph.target(edge);

        if (source === selectedNode || target === selectedNode) {
            res.color = '#0969da';
            res.size = 2;
        }
    }

    return res;
}

// =============================================================================
// Event Handlers
// =============================================================================

/**
 * Setup all event handlers for interaction.
 */
function setupEventHandlers() {
    // Node click - select and show details
    renderer.on('clickNode', ({ node }) => {
        selectedNode = node;
        showNodeDetails(node);
        renderer.refresh();
    });

    // Stage click - deselect
    renderer.on('clickStage', () => {
        selectedNode = null;
        hideNodeDetails();
        renderer.refresh();
    });

    // Node hover - highlight neighbors
    renderer.on('enterNode', ({ node }) => {
        hoveredNode = node;
        renderer.refresh();
    });

    renderer.on('leaveNode', () => {
        hoveredNode = null;
        renderer.refresh();
    });

    // Search input
    document.getElementById('search').addEventListener('input', (e) => {
        searchQuery = e.target.value;
        renderer.refresh();
    });

    // Note: Type filter checkboxes are created dynamically in buildFilterUI()

    // Layout toggle button
    document.getElementById('layout-btn').addEventListener('click', toggleLayout);

    // Reset view button
    document.getElementById('reset-btn').addEventListener('click', () => {
        renderer.getCamera().animatedReset();
    });
}

// =============================================================================
// Node Details Panel
// =============================================================================

/**
 * Show the node details panel for a node.
 */
function showNodeDetails(nodeKey) {
    const attrs = graph.getNodeAttributes(nodeKey);

    document.getElementById('detail-name').textContent = attrs.label;
    document.getElementById('detail-type').textContent = attrs.category;
    document.getElementById('detail-file').textContent = attrs.file || '-';
    document.getElementById('detail-lines').textContent =
        attrs.start_line ? attrs.start_line + '-' + attrs.end_line : '-';
    document.getElementById('detail-outgoing').textContent = graph.outDegree(nodeKey);
    document.getElementById('detail-incoming').textContent = graph.inDegree(nodeKey);

    document.getElementById('node-details').classList.add('visible');
}

/**
 * Hide the node details panel.
 */
function hideNodeDetails() {
    document.getElementById('node-details').classList.remove('visible');
}

// =============================================================================
// Layout Control
// =============================================================================

/**
 * Toggle the ForceAtlas2 layout on/off.
 */
function toggleLayout() {
    if (layoutRunning) {
        stopLayout();
    } else {
        fa2Layout.start();
        layoutRunning = true;
        updateLayoutButton(true);
    }
}

/**
 * Stop the layout algorithm.
 */
function stopLayout() {
    fa2Layout.stop();
    layoutRunning = false;
    updateLayoutButton(false);
}

/**
 * Update the layout button state.
 */
function updateLayoutButton(running) {
    const btn = document.getElementById('layout-btn');
    btn.textContent = running ? 'Stop Layout' : 'Start Layout';
    btn.classList.toggle('active', running);
}

// =============================================================================
// Start Application
// =============================================================================

init();
