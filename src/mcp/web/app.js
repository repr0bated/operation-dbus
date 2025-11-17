// MCP Control Center Application
class MCPControlCenter {
    constructor() {
        this.ws = null;
        this.tools = [];
        this.agents = [];
        this.services = [];
        this.logs = [];
        this.stats = {
            uptime: 0,
            requestCount: 0,
            activeAgents: 0,
            availableTools: 0
        };
        this.currentTool = null;
        this.activityFeed = [];
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        
        this.init();
    }

    init() {
        this.setupWebSocket();
        this.setupEventListeners();
        this.setupNavigation();
        this.loadInitialData();
        this.startUpdateTimers();
        this.checkTheme();
    }

    // WebSocket Connection
    setupWebSocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;
        
        this.ws = new WebSocket(wsUrl);
        
        this.ws.onopen = () => {
            console.log('WebSocket connected');
            this.updateConnectionStatus('connected');
            this.reconnectAttempts = 0;
            this.addActivity('Connected to MCP server');
        };
        
        this.ws.onmessage = (event) => {
            this.handleWebSocketMessage(JSON.parse(event.data));
        };
        
        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            this.updateConnectionStatus('error');
        };
        
        this.ws.onclose = () => {
            console.log('WebSocket disconnected');
            this.updateConnectionStatus('disconnected');
            this.addActivity('Disconnected from MCP server', 'error');
            this.attemptReconnect();
        };
    }

    attemptReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
            console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
            setTimeout(() => this.setupWebSocket(), delay);
        } else {
            this.showToast('Connection lost. Please refresh the page.', 'error');
        }
    }

    handleWebSocketMessage(data) {
        switch (data.type) {
            case 'status':
                this.updateStats(data.data);
                break;
            case 'agent_update':
                this.updateAgents(data.data);
                break;
            case 'log':
                this.addLog(data.data);
                break;
            case 'activity':
                this.addActivity(data.message, data.level);
                break;
            case 'tool_result':
                this.handleToolResult(data.data);
                break;
            default:
                console.log('Unknown message type:', data.type);
        }
    }

    // Event Listeners
    setupEventListeners() {
        // Theme toggle
        document.getElementById('theme-toggle').addEventListener('click', () => {
            this.toggleTheme();
        });

        // Dashboard refresh
        document.getElementById('refresh-dashboard').addEventListener('click', () => {
            this.refreshDashboard();
        });

        // Tool search
        document.getElementById('tool-search').addEventListener('input', (e) => {
            this.filterTools(e.target.value);
        });

        // Log level filter
        document.getElementById('log-level').addEventListener('change', (e) => {
            this.filterLogs(e.target.value);
        });
    }

    // Navigation
    setupNavigation() {
        const navLinks = document.querySelectorAll('.nav-link');
        navLinks.forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const target = link.getAttribute('href').substring(1);
                this.navigateTo(target);
            });
        });
    }

    navigateTo(section) {
        // Update nav
        document.querySelectorAll('.nav-link').forEach(link => {
            link.classList.remove('active');
        });
        document.querySelector(`.nav-link[href="#${section}"]`).classList.add('active');

        // Update sections
        document.querySelectorAll('.section').forEach(sec => {
            sec.classList.remove('active');
        });
        document.getElementById(section).classList.add('active');

        // Load section-specific data
        this.loadSectionData(section);
    }

    loadSectionData(section) {
        switch (section) {
            case 'tools':
                this.loadTools();
                break;
            case 'agents':
                this.loadAgents();
                break;
            case 'discovery':
                this.loadServices();
                break;
            case 'logs':
                this.loadLogs();
                break;
        }
    }

    // Initial Data Load
    async loadInitialData() {
        await this.loadStatus();
        await this.loadTools();
        await this.loadAgents();
    }

    async loadStatus() {
        try {
            const response = await fetch('/api/status');
            const data = await response.json();
            if (data.success) {
                this.updateStats(data.data);
            }
        } catch (error) {
            console.error('Failed to load status:', error);
        }
    }

    async loadTools() {
        try {
            const response = await fetch('/api/tools');
            const data = await response.json();
            if (data.success) {
                this.tools = data.data.tools || [];
                this.renderTools();
                this.stats.availableTools = this.tools.length;
                this.updateStatDisplay('available-tools', this.tools.length);
            }
        } catch (error) {
            console.error('Failed to load tools:', error);
        }
    }

    async loadAgents() {
        try {
            const response = await fetch('/api/agents');
            const data = await response.json();
            if (data.success) {
                this.agents = data.data || [];
                this.renderAgents();
                this.stats.activeAgents = this.agents.length;
                this.updateStatDisplay('active-agents', this.agents.length);
            }
        } catch (error) {
            console.error('Failed to load agents:', error);
        }
    }

    async loadServices() {
        try {
            const response = await fetch('/api/discovery/services');
            const data = await response.json();
            if (data.success) {
                this.services = data.data || [];
                this.renderDiscoveryResults();
            }
        } catch (error) {
            console.error('Failed to load services:', error);
        }
    }

    async loadLogs() {
        // In a real implementation, this would fetch historical logs
        this.renderLogs();
    }

    // Rendering Functions
    renderTools() {
        const grid = document.getElementById('tools-grid');
        grid.innerHTML = '';

        this.tools.forEach(tool => {
            const card = document.createElement('div');
            card.className = 'tool-card';
            card.innerHTML = `
                <div class="tool-name">${this.escapeHtml(tool.name)}</div>
                <div class="tool-description">${this.escapeHtml(tool.description || 'No description')}</div>
                <div class="tool-meta">
                    ${tool.inputSchema?.required?.map(req => 
                        `<span class="tool-tag">Required: ${req}</span>`
                    ).join('') || ''}
                </div>
            `;
            card.addEventListener('click', () => this.openToolTest(tool));
            grid.appendChild(card);
        });
    }

    renderAgents() {
        const tbody = document.getElementById('agents-tbody');
        tbody.innerHTML = '';

        if (this.agents.length === 0) {
            tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;">No active agents</td></tr>';
            return;
        }

        this.agents.forEach(agent => {
            const row = document.createElement('tr');
            row.innerHTML = `
                <td><code>${this.escapeHtml(agent.id)}</code></td>
                <td>${this.escapeHtml(agent.agent_type)}</td>
                <td><span class="badge badge-${agent.status === 'running' ? 'success' : 'warning'}">${agent.status}</span></td>
                <td>${agent.task || '-'}</td>
                <td>${this.formatUptime(agent.uptime || 0)}</td>
                <td>
                    <button class="btn btn-sm" onclick="mcp.sendTaskToAgent('${agent.id}')">Task</button>
                    <button class="btn btn-sm btn-danger" onclick="mcp.killAgent('${agent.id}')">Kill</button>
                </td>
            `;
            tbody.appendChild(row);
        });
    }

    renderDiscoveryResults() {
        const container = document.getElementById('discovery-results');
        const categories = this.groupServicesByCategory();
        
        container.innerHTML = `
            <div class="discovery-categories">
                ${Object.entries(categories).map(([category, services]) => `
                    <div class="category-section">
                        <h3>${category} (${services.length})</h3>
                        <div class="service-list">
                            ${services.map(service => `
                                <div class="service-item">
                                    <span class="service-name">${this.escapeHtml(service.name)}</span>
                                    <span class="service-path">${this.escapeHtml(service.path || '')}</span>
                                </div>
                            `).join('')}
                        </div>
                    </div>
                `).join('')}
            </div>
        `;
    }

    // Enhanced Discovery Methods
    async discoverServices() {
        this.showToast('Discovering D-Bus services...', 'info');

        try {
            const response = await fetch('/api/tools/list_dbus_services/execute', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ include_activatable: false })
            });

            const data = await response.json();

            if (data.success && data.result.active_services) {
                this.discoveredServices = data.result.active_services;
                this.serviceTree = {};  // Store expanded state
                this.showToast(`Discovered ${this.discoveredServices.length} services`, 'success');

                // Update stats
                document.getElementById('discovery-stats').style.display = 'block';
                document.getElementById('stat-services').textContent = this.discoveredServices.length;

                this.renderDiscoveryTree();
            } else {
                this.showToast('Discovery failed', 'error');
            }
        } catch (error) {
            console.error('Discovery error:', error);
            this.showToast('Failed to discover services', 'error');
        }
    }

    async expandService(serviceName) {
        if (this.serviceTree[serviceName]?.paths) {
            // Already loaded, just toggle
            return;
        }

        try {
            const response = await fetch('/api/tools/list_dbus_object_paths/execute', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ service_name: serviceName })
            });

            const data = await response.json();

            if (data.success && data.result.object_paths) {
                if (!this.serviceTree[serviceName]) {
                    this.serviceTree[serviceName] = {};
                }
                this.serviceTree[serviceName].paths = data.result.object_paths;
                this.serviceTree[serviceName].expanded = true;

                // Update object count
                const totalObjects = Object.values(this.serviceTree)
                    .filter(s => s.paths)
                    .reduce((sum, s) => sum + s.paths.length, 0);
                document.getElementById('stat-objects').textContent = totalObjects;

                this.renderDiscoveryTree();
            }
        } catch (error) {
            console.error('Failed to expand service:', error);
            this.showToast(`Failed to load paths for ${serviceName}`, 'error');
        }
    }

    async expandObject(serviceName, objectPath) {
        const key = `${serviceName}:${objectPath}`;

        if (this.serviceTree[serviceName]?.objects?.[objectPath]) {
            // Already loaded
            return;
        }

        try {
            const response = await fetch('/api/tools/introspect_dbus_object/execute', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    service_name: serviceName,
                    object_path: objectPath
                })
            });

            const data = await response.json();

            if (data.success && data.result.interfaces) {
                if (!this.serviceTree[serviceName].objects) {
                    this.serviceTree[serviceName].objects = {};
                }
                this.serviceTree[serviceName].objects[objectPath] = {
                    interfaces: data.result.interfaces,
                    child_nodes: data.result.child_nodes || [],
                    expanded: true
                };

                // Update interface and method counts
                let totalInterfaces = 0;
                let totalMethods = 0;
                Object.values(this.serviceTree).forEach(service => {
                    if (service.objects) {
                        Object.values(service.objects).forEach(obj => {
                            totalInterfaces += obj.interfaces.length;
                            obj.interfaces.forEach(iface => {
                                totalMethods += iface.methods?.length || 0;
                            });
                        });
                    }
                });
                document.getElementById('stat-interfaces').textContent = totalInterfaces;
                document.getElementById('stat-methods').textContent = totalMethods;

                this.renderDiscoveryTree();
            }
        } catch (error) {
            console.error('Failed to introspect object:', error);
            this.showToast(`Failed to introspect ${objectPath}`, 'error');
        }
    }

    renderDiscoveryTree() {
        const container = document.getElementById('discovery-results');
        const viewMode = document.getElementById('discovery-view-mode')?.value || 'tree';

        if (!this.discoveredServices || this.discoveredServices.length === 0) {
            return;
        }

        let html = '<div class="discovery-tree">';

        this.discoveredServices.forEach(serviceName => {
            const service = this.serviceTree[serviceName] || {};
            const isExpanded = service.expanded;
            const paths = service.paths || [];

            html += `
                <div class="tree-node service-node">
                    <div class="tree-node-header" onclick="window.mcp.toggleService('${this.escapeHtml(serviceName)}')">
                        <span class="tree-toggle">${paths.length > 0 ? (isExpanded ? '▼' : '►') : '○'}</span>
                        <span class="tree-icon">📦</span>
                        <span class="tree-label">${this.escapeHtml(serviceName)}</span>
                        ${paths.length > 0 ? `<span class="tree-badge">${paths.length} objects</span>` : ''}
                        <button class="btn btn-xs" onclick="event.stopPropagation(); window.mcp.introspectServiceManually('${this.escapeHtml(serviceName)}')" style="margin-left: auto;">
                            Introspect
                        </button>
                    </div>
                    ${isExpanded && paths.length > 0 ? `
                        <div class="tree-children">
                            ${paths.map(path => this.renderObjectNode(serviceName, path)).join('')}
                        </div>
                    ` : ''}
                </div>
            `;
        });

        html += '</div>';
        container.innerHTML = html;
    }

    renderObjectNode(serviceName, objectPath) {
        const service = this.serviceTree[serviceName];
        const object = service?.objects?.[objectPath];
        const isExpanded = object?.expanded;
        const interfaces = object?.interfaces || [];

        let html = `
            <div class="tree-node object-node">
                <div class="tree-node-header" onclick="window.mcp.toggleObject('${this.escapeHtml(serviceName)}', '${this.escapeHtml(objectPath)}')">
                    <span class="tree-toggle">${interfaces.length > 0 ? (isExpanded ? '▼' : '►') : '○'}</span>
                    <span class="tree-icon">📄</span>
                    <span class="tree-label">${this.escapeHtml(objectPath)}</span>
                    ${interfaces.length > 0 ? `<span class="tree-badge">${interfaces.length} interfaces</span>` : ''}
                </div>
                ${isExpanded && interfaces.length > 0 ? `
                    <div class="tree-children">
                        ${interfaces.map(iface => this.renderInterfaceNode(iface)).join('')}
                    </div>
                ` : ''}
            </div>
        `;

        return html;
    }

    renderInterfaceNode(iface) {
        const methods = iface.methods || [];
        const properties = iface.properties || [];
        const signals = iface.signals || [];

        return `
            <div class="tree-node interface-node">
                <div class="tree-node-header">
                    <span class="tree-icon">⚡</span>
                    <span class="tree-label">${this.escapeHtml(iface.name)}</span>
                    ${methods.length > 0 ? `<span class="tree-badge badge-method">${methods.length} methods</span>` : ''}
                    ${properties.length > 0 ? `<span class="tree-badge badge-property">${properties.length} props</span>` : ''}
                    ${signals.length > 0 ? `<span class="tree-badge badge-signal">${signals.length} signals</span>` : ''}
                </div>
                <div class="tree-children">
                    ${methods.map(m => `
                        <div class="tree-node method-node">
                            <span class="tree-icon">🔧</span>
                            <span class="tree-label">${this.escapeHtml(m.name)}(${m.in_args?.map(a => a.type).join(', ') || ''})</span>
                            ${m.out_args?.length > 0 ? `<span class="tree-type">→ ${m.out_args.map(a => a.type).join(', ')}</span>` : ''}
                        </div>
                    `).join('')}
                    ${properties.map(p => `
                        <div class="tree-node property-node">
                            <span class="tree-icon">📋</span>
                            <span class="tree-label">${this.escapeHtml(p.name)}</span>
                            <span class="tree-type">${p.type}</span>
                            <span class="tree-access">${p.access}</span>
                        </div>
                    `).join('')}
                    ${signals.map(s => `
                        <div class="tree-node signal-node">
                            <span class="tree-icon">📡</span>
                            <span class="tree-label">${this.escapeHtml(s.name)}</span>
                        </div>
                    `).join('')}
                </div>
            </div>
        `;
    }

    async toggleService(serviceName) {
        const service = this.serviceTree[serviceName];

        if (!service || !service.paths) {
            // First time - load paths
            await this.expandService(serviceName);
        } else {
            // Toggle expansion
            service.expanded = !service.expanded;
            this.renderDiscoveryTree();
        }
    }

    async toggleObject(serviceName, objectPath) {
        const service = this.serviceTree[serviceName];
        const object = service?.objects?.[objectPath];

        if (!object) {
            // First time - load introspection
            await this.expandObject(serviceName, objectPath);
        } else {
            // Toggle expansion
            object.expanded = !object.expanded;
            this.renderDiscoveryTree();
        }
    }

    async expandAllServices() {
        if (!this.discoveredServices) {
            this.showToast('Run discovery first', 'warning');
            return;
        }

        this.showToast('Expanding all services...', 'info');

        for (const serviceName of this.discoveredServices) {
            if (!this.serviceTree[serviceName]?.paths) {
                await this.expandService(serviceName);
            } else {
                this.serviceTree[serviceName].expanded = true;
            }
        }

        this.renderDiscoveryTree();
        this.showToast('All services expanded', 'success');
    }

    collapseAllServices() {
        if (!this.serviceTree) return;

        Object.values(this.serviceTree).forEach(service => {
            service.expanded = false;
            if (service.objects) {
                Object.values(service.objects).forEach(obj => {
                    obj.expanded = false;
                });
            }
        });

        this.renderDiscoveryTree();
    }

    filterServices(query) {
        // TODO: Implement filtering
        console.log('Filter:', query);
    }

    changeDiscoveryView(viewMode) {
        // TODO: Implement different view modes
        console.log('View mode:', viewMode);
    }

    introspectServiceManually(serviceName) {
        const path = prompt(`Enter object path for ${serviceName}:`, '/');
        if (path) {
            this.expandObject(serviceName, path);
        }
    }

    renderLogs() {
        const container = document.getElementById('logs-container');
        container.innerHTML = '';

        this.logs.forEach(log => {
            const entry = document.createElement('div');
            entry.className = `log-entry log-${log.level}`;
            entry.innerHTML = `
                <span class="log-timestamp">${this.formatTime(log.timestamp)}</span>
                <span class="log-level ${log.level}">${log.level.toUpperCase()}</span>
                <span class="log-message">${this.escapeHtml(log.message)}</span>
            `;
            container.appendChild(entry);
        });

        // Auto-scroll to bottom
        container.scrollTop = container.scrollHeight;
    }

    // Tool Testing
    openToolTest(tool) {
        this.currentTool = tool;
        document.getElementById('test-tool-name').textContent = tool.name;
        
        // Set default parameters based on schema
        const params = {};
        if (tool.inputSchema?.properties) {
            Object.entries(tool.inputSchema.properties).forEach(([key, schema]) => {
                if (schema.default !== undefined) {
                    params[key] = schema.default;
                } else if (schema.type === 'string') {
                    params[key] = '';
                } else if (schema.type === 'number') {
                    params[key] = 0;
                } else if (schema.type === 'boolean') {
                    params[key] = false;
                } else if (schema.type === 'array') {
                    params[key] = [];
                } else if (schema.type === 'object') {
                    params[key] = {};
                }
            });
        }
        
        document.getElementById('tool-params').value = JSON.stringify(params, null, 2);
        document.getElementById('tool-result').textContent = '';
        document.getElementById('tool-test-panel').style.display = 'block';
    }

    closeToolTest() {
        document.getElementById('tool-test-panel').style.display = 'none';
        this.currentTool = null;
    }

    clearToolTest() {
        document.getElementById('tool-params').value = '{}';
        document.getElementById('tool-result').textContent = '';
    }

    async executeToolTest() {
        if (!this.currentTool) return;

        const paramsText = document.getElementById('tool-params').value;
        let params;
        try {
            params = JSON.parse(paramsText);
        } catch (e) {
            this.showToast('Invalid JSON parameters', 'error');
            return;
        }

        const resultEl = document.getElementById('tool-result');
        resultEl.textContent = 'Executing...';

        try {
            const response = await fetch(`/api/tools/${this.currentTool.name}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(params)
            });
            
            const data = await response.json();
            
            if (data.success) {
                resultEl.textContent = JSON.stringify(data.data, null, 2);
                this.showToast('Tool executed successfully', 'success');
            } else {
                resultEl.textContent = `Error: ${data.error}`;
                this.showToast(`Execution failed: ${data.error}`, 'error');
            }
        } catch (error) {
            resultEl.textContent = `Error: ${error.message}`;
            this.showToast('Failed to execute tool', 'error');
        }
    }

    // Agent Management
    showSpawnAgent() {
        document.getElementById('spawn-agent-modal').style.display = 'flex';
    }

    hideSpawnAgent() {
        document.getElementById('spawn-agent-modal').style.display = 'none';
    }

    async spawnAgent() {
        const agentType = document.getElementById('agent-type').value;
        const configText = document.getElementById('agent-config').value;
        
        let config;
        try {
            config = JSON.parse(configText || '{}');
        } catch (e) {
            this.showToast('Invalid JSON configuration', 'error');
            return;
        }

        try {
            const response = await fetch('/api/agents', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ type: agentType, config })
            });
            
            const data = await response.json();
            
            if (data.success) {
                this.showToast(`Agent spawned: ${data.data.agent_id}`, 'success');
                this.hideSpawnAgent();
                this.loadAgents();
            } else {
                this.showToast(`Failed to spawn agent: ${data.error}`, 'error');
            }
        } catch (error) {
            this.showToast('Failed to spawn agent', 'error');
        }
    }

    async killAgent(agentId) {
        if (!confirm('Are you sure you want to kill this agent?')) return;

        try {
            const response = await fetch(`/api/agents/${agentId}`, {
                method: 'DELETE'
            });
            
            const data = await response.json();
            
            if (data.success) {
                this.showToast('Agent terminated', 'success');
                this.loadAgents();
            } else {
                this.showToast(`Failed to kill agent: ${data.error}`, 'error');
            }
        } catch (error) {
            this.showToast('Failed to kill agent', 'error');
        }
    }

    // Discovery
    async runDiscovery() {
        this.showToast('Running service discovery...', 'info');
        
        try {
            const response = await fetch('/api/discovery/run', {
                method: 'POST'
            });
            
            const data = await response.json();
            
            if (data.success) {
                this.showToast(`Discovered ${data.data.count} services`, 'success');
                this.loadServices();
            } else {
                this.showToast(`Discovery failed: ${data.error}`, 'error');
            }
        } catch (error) {
            this.showToast('Failed to run discovery', 'error');
        }
    }

    groupServicesByCategory() {
        const categories = {};
        
        this.services.forEach(service => {
            const category = service.category || 'Other';
            if (!categories[category]) {
                categories[category] = [];
            }
            categories[category].push(service);
        });
        
        return categories;
    }

    // Update Functions
    updateStats(stats) {
        this.stats = { ...this.stats, ...stats };
        
        this.updateStatDisplay('uptime', this.formatUptime(stats.uptime_secs));
        this.updateStatDisplay('request-count', stats.request_count);
        this.updateStatDisplay('active-agents', stats.active_agents?.length || 0);
    }

    updateStatDisplay(id, value) {
        const element = document.getElementById(id);
        if (element) {
            element.textContent = value;
        }
    }

    updateConnectionStatus(status) {
        const statusEl = document.getElementById('connection-status');
        statusEl.className = `connection-status ${status}`;
        
        const statusText = statusEl.querySelector('.status-text');
        switch (status) {
            case 'connected':
                statusText.textContent = 'Connected';
                break;
            case 'disconnected':
                statusText.textContent = 'Disconnected';
                break;
            case 'error':
                statusText.textContent = 'Error';
                break;
            default:
                statusText.textContent = 'Connecting...';
        }
    }

    updateAgents(agents) {
        this.agents = agents;
        if (document.querySelector('.section#agents.active')) {
            this.renderAgents();
        }
        this.updateStatDisplay('active-agents', agents.length);
    }

    // Activity Feed
    addActivity(message, level = 'info') {
        const activity = {
            timestamp: new Date(),
            message,
            level
        };
        
        this.activityFeed.unshift(activity);
        if (this.activityFeed.length > 100) {
            this.activityFeed.pop();
        }
        
        this.renderActivityFeed();
    }

    renderActivityFeed() {
        const container = document.getElementById('activity-feed');
        
        const html = this.activityFeed.slice(0, 20).map(item => `
            <div class="feed-item">
                <span class="feed-time">${this.formatTime(item.timestamp)}</span>
                <span class="feed-message">${this.escapeHtml(item.message)}</span>
            </div>
        `).join('');
        
        container.innerHTML = html;
    }

    // Logs
    addLog(log) {
        this.logs.push({
            timestamp: new Date(),
            ...log
        });
        
        if (this.logs.length > 1000) {
            this.logs.shift();
        }
        
        if (document.querySelector('.section#logs.active')) {
            this.renderLogs();
        }
    }

    filterLogs(level) {
        // Implementation for log filtering
        const filtered = level === 'all' 
            ? this.logs 
            : this.logs.filter(log => log.level === level);
        
        // Re-render with filtered logs
        const tempLogs = this.logs;
        this.logs = filtered;
        this.renderLogs();
        this.logs = tempLogs;
    }

    clearLogs() {
        this.logs = [];
        this.renderLogs();
    }

    downloadLogs() {
        const content = this.logs.map(log => 
            `[${this.formatTime(log.timestamp)}] [${log.level}] ${log.message}`
        ).join('\n');
        
        const blob = new Blob([content], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `mcp-logs-${Date.now()}.txt`;
        a.click();
        URL.revokeObjectURL(url);
    }

    // Theme Management
    checkTheme() {
        const savedTheme = localStorage.getItem('theme') || 'dark';
        document.documentElement.setAttribute('data-theme', savedTheme);
        this.updateThemeToggle(savedTheme);
    }

    toggleTheme() {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
        
        document.documentElement.setAttribute('data-theme', newTheme);
        localStorage.setItem('theme', newTheme);
        this.updateThemeToggle(newTheme);
    }

    updateThemeToggle(theme) {
        const sunIcon = document.querySelector('.sun-icon');
        const moonIcon = document.querySelector('.moon-icon');
        
        if (theme === 'dark') {
            sunIcon.style.display = 'block';
            moonIcon.style.display = 'none';
        } else {
            sunIcon.style.display = 'none';
            moonIcon.style.display = 'block';
        }
    }

    // Utilities
    formatUptime(seconds) {
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;
        return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }

    formatTime(date) {
        if (!(date instanceof Date)) {
            date = new Date(date);
        }
        return date.toLocaleTimeString('en-US', { hour12: false });
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    showToast(message, type = 'info') {
        const container = document.getElementById('toast-container');
        
        const toast = document.createElement('div');
        toast.className = `toast ${type}`;
        toast.innerHTML = `
            <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
                ${this.getToastIcon(type)}
            </svg>
            <span>${this.escapeHtml(message)}</span>
        `;
        
        container.appendChild(toast);
        
        setTimeout(() => {
            toast.style.animation = 'slideOut 0.3s ease';
            setTimeout(() => toast.remove(), 300);
        }, 5000);
    }

    getToastIcon(type) {
        switch (type) {
            case 'success':
                return '<path d="M10 0C4.5 0 0 4.5 0 10s4.5 10 10 10 10-4.5 10-10S15.5 0 10 0zm4 8l-5 5-3-3 1.5-1.5L9 10l3.5-3.5L14 8z"/>';
            case 'error':
                return '<path d="M10 0C4.5 0 0 4.5 0 10s4.5 10 10 10 10-4.5 10-10S15.5 0 10 0zm5 13.5L13.5 15 10 11.5 6.5 15 5 13.5 8.5 10 5 6.5 6.5 5 10 8.5 13.5 5 15 6.5 11.5 10 15 13.5z"/>';
            case 'warning':
                return '<path d="M10 0C4.5 0 0 4.5 0 10s4.5 10 10 10 10-4.5 10-10S15.5 0 10 0zm0 15c-.6 0-1-.4-1-1s.4-1 1-1 1 .4 1 1-.4 1-1 1zm1-4H9V5h2v6z"/>';
            default:
                return '<path d="M10 0C4.5 0 0 4.5 0 10s4.5 10 10 10 10-4.5 10-10S15.5 0 10 0zm1 15H9v-2h2v2zm0-4H9V5h2v6z"/>';
        }
    }

    refreshDashboard() {
        this.showToast('Refreshing dashboard...', 'info');
        this.loadStatus();
        this.loadAgents();
    }

    startUpdateTimers() {
        // Update uptime every second
        setInterval(() => {
            this.stats.uptime_secs++;
            this.updateStatDisplay('uptime', this.formatUptime(this.stats.uptime_secs));
        }, 1000);

        // Refresh stats every 10 seconds
        setInterval(() => {
            this.loadStatus();
        }, 10000);
    }
}

// Global instance
window.mcp = new MCPControlCenter();

// Global functions for inline handlers
window.closeToolTest = () => window.mcp.closeToolTest();
window.clearToolTest = () => window.mcp.clearToolTest();
window.executeToolTest = () => window.mcp.executeToolTest();
window.showSpawnAgent = () => window.mcp.showSpawnAgent();
window.hideSpawnAgent = () => window.mcp.hideSpawnAgent();
window.spawnAgent = () => window.mcp.spawnAgent();
window.runDiscovery = () => window.mcp.runDiscovery();
window.clearLogs = () => window.mcp.clearLogs();
window.downloadLogs = () => window.mcp.downloadLogs();