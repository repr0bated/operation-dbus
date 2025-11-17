#!/usr/bin/env node
/**
 * MCP Server for operation-dbus
 * Implements Model Context Protocol over stdio
 * Wraps existing operation-dbus tools from chat-server
 */

const axios = require('axios');
const readline = require('readline');

const CHAT_SERVER_URL = 'http://localhost:8080';

class MCPServer {
    constructor() {
        this.tools = [];
        this.initialized = false;
        this.rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout,
            terminal: false
        });
    }

    async fetchTools() {
        try {
            const response = await axios.get(`${CHAT_SERVER_URL}/api/tools`);
            // Handle the actual response format from chat server
            let toolsArray;
            if (response.data.tools) {
                // Response has nested structure: {tools: [...]}
                toolsArray = response.data.tools;
            } else if (Array.isArray(response.data)) {
                // Response is direct array
                toolsArray = response.data;
            } else {
                console.error('[MCP] Unexpected response format:', response.data);
                return [];
            }

            this.tools = toolsArray.map(tool => ({
                name: tool.name,
                description: tool.description,
                inputSchema: {
                    type: 'object',
                    properties: tool.parameters || {},
                    required: []
                }
            }));
            console.error(`[MCP] Fetched ${this.tools.length} tools from chat server`);
            return this.tools;
        } catch (error) {
            console.error('[MCP] Failed to fetch tools:', error.message);
            return [];
        }
    }

    async handleRequest(request) {
        const { jsonrpc, id, method, params } = request;

        try {
            let result;

            switch (method) {
                case 'initialize':
                    // Don't fetch tools on initialize to avoid blocking
                    // Tools will be fetched on first tools/list call
                    this.initialized = true;

                    result = {
                        protocolVersion: '2024-11-05',
                        serverInfo: {
                            name: 'operation-dbus',
                            version: '0.1.0'
                        },
                        capabilities: {
                            tools: {}
                        }
                    };
                    console.error('[MCP] Initialized successfully');
                    break;

                case 'tools/list':
                    if (!this.initialized) {
                        await this.fetchTools();
                        this.initialized = true;
                    }
                    result = {
                        tools: this.tools
                    };
                    break;

                case 'tools/call':
                    result = await this.executeTool(params.name, params.arguments || {});
                    break;

                default:
                    throw new Error(`Unknown method: ${method}`);
            }

            return {
                jsonrpc: '2.0',
                id,
                result
            };

        } catch (error) {
            return {
                jsonrpc: '2.0',
                id,
                error: {
                    code: -32603,
                    message: error.message
                }
            };
        }
    }

    async executeTool(toolName, args) {
        console.error(`[MCP] Executing tool: ${toolName}`);

        try {
            const response = await axios.post(`${CHAT_SERVER_URL}/api/tools/execute`, {
                tool: toolName,
                parameters: args
            }, {
                timeout: 30000
            });

            if (response.data.success) {
                const result = response.data.result;
                const resultText = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
                
                return {
                    content: [{
                        type: 'text',
                        text: resultText
                    }]
                };
            } else {
                throw new Error(response.data.error || 'Tool execution failed');
            }
        } catch (error) {
            console.error(`[MCP] Error:`, error.message);
            return {
                content: [{
                    type: 'text',
                    text: `Error: ${error.message}`
                }],
                isError: true
            };
        }
    }

    start() {
        console.error('[MCP] operation-dbus MCP server starting');
        console.error('[MCP] Protocol: stdio (for Cursor/Claude/DeepSeek)');

        // Handle stdin data directly for better MCP compatibility
        let buffer = '';
        process.stdin.on('data', async (chunk) => {
            buffer += chunk.toString();
            const lines = buffer.split('\n');
            buffer = lines.pop() || ''; // Keep incomplete line

            for (const line of lines) {
                if (!line.trim()) continue;

                try {
                    console.error(`[MCP] Received: ${line}`);
                    const request = JSON.parse(line);
                    const response = await this.handleRequest(request);
                    console.log(JSON.stringify(response));
                } catch (error) {
                    console.error(`[MCP] Parse error: ${error.message}`);
                    console.log(JSON.stringify({
                        jsonrpc: '2.0',
                        id: null,
                        error: {
                            code: -32700,
                            message: 'Parse error'
                        }
                    }));
                }
            }
        });

        process.stdin.on('end', () => {
            console.error('[MCP] Stdin ended, waiting for operations to complete...');
            // Give async operations time to complete
            setTimeout(() => {
                console.error('[MCP] Server shutting down');
                process.exit(0);
            }, 1000);
        });

        // Also keep the readline interface as backup
        this.rl.on('line', async (line) => {
            if (!line.trim()) return;

            try {
                const request = JSON.parse(line);
                const response = await this.handleRequest(request);
                console.log(JSON.stringify(response));
            } catch (error) {
                console.error(`[MCP] Parse error: ${error.message}`);
                console.log(JSON.stringify({
                    jsonrpc: '2.0',
                    id: null,
                    error: {
                        code: -32700,
                        message: 'Parse error'
                    }
                }));
            }
        });

        this.rl.on('close', () => {
            console.error('[MCP] Readline interface closed');
            // Don't exit here since we have direct stdin handling
        });
    }
}

const server = new MCPServer();
server.start();
