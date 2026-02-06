#!/usr/bin/env python3
"""
MCP Server for Gemini API via Antigravity Proxy
Provides Gemini capabilities through Model Context Protocol
"""
import os
import json
import asyncio
from typing import Any, Optional
from mcp.server import Server
from mcp.types import Tool, TextContent
from openai import AsyncOpenAI

# Configuration
GEMINI_BASE_URL = os.getenv("GEMINI_BASE_URL", "http://127.0.0.1:8045/v1")
GEMINI_API_KEY = os.getenv("GEMINI_API_KEY", "sk-28da1542217448069593b22690c561ca")
GEMINI_MODEL = os.getenv("GEMINI_MODEL", "gemini-2.0-flash-exp")

# Initialize OpenAI client pointing to Antigravity proxy
client = AsyncOpenAI(
    base_url=GEMINI_BASE_URL,
    api_key=GEMINI_API_KEY
)

# Create MCP server
app = Server("gemini-proxy")

@app.list_tools()
async def list_tools() -> list[Tool]:
    """List available Gemini tools"""
    return [
        Tool(
            name="gemini_chat",
            description="Chat with Gemini AI model through Antigravity proxy",
            inputSchema={
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Message to send to Gemini"
                    },
                    "model": {
                        "type": "string",
                        "description": "Model to use (default: gemini-2.0-flash-exp)",
                        "enum": ["gemini-2.0-flash-exp", "gemini-1.5-pro", "gemini-1.5-flash"]
                    },
                    "system": {
                        "type": "string",
                        "description": "System prompt (optional)"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "Temperature 0-2 (default: 1.0)"
                    }
                },
                "required": ["message"]
            }
        ),
        Tool(
            name="gemini_analyze_code",
            description="Analyze code using Gemini's code understanding",
            inputSchema={
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Code to analyze"
                    },
                    "language": {
                        "type": "string",
                        "description": "Programming language"
                    },
                    "task": {
                        "type": "string",
                        "description": "What to do (review, explain, optimize, debug)",
                        "enum": ["review", "explain", "optimize", "debug", "document"]
                    }
                },
                "required": ["code", "task"]
            }
        ),
        Tool(
            name="gemini_generate_code",
            description="Generate code using Gemini",
            inputSchema={
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "What code to generate"
                    },
                    "language": {
                        "type": "string",
                        "description": "Programming language"
                    },
                    "context": {
                        "type": "string",
                        "description": "Additional context (optional)"
                    }
                },
                "required": ["description", "language"]
            }
        )
    ]

@app.call_tool()
async def call_tool(name: str, arguments: Any) -> list[TextContent]:
    """Handle tool calls"""
    
    if name == "gemini_chat":
        return await gemini_chat(arguments)
    elif name == "gemini_analyze_code":
        return await gemini_analyze_code(arguments)
    elif name == "gemini_generate_code":
        return await gemini_generate_code(arguments)
    else:
        return [TextContent(type="text", text=f"Unknown tool: {name}")]

async def gemini_chat(args: dict) -> list[TextContent]:
    """Chat with Gemini"""
    message = args["message"]
    model = args.get("model", GEMINI_MODEL)
    system = args.get("system")
    temperature = args.get("temperature", 1.0)
    
    messages = []
    if system:
        messages.append({"role": "system", "content": system})
    messages.append({"role": "user", "content": message})
    
    try:
        response = await client.chat.completions.create(
            model=model,
            messages=messages,
            temperature=temperature
        )
        
        content = response.choices[0].message.content
        return [TextContent(type="text", text=content)]
    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]

async def gemini_analyze_code(args: dict) -> list[TextContent]:
    """Analyze code with Gemini"""
    code = args["code"]
    language = args.get("language", "unknown")
    task = args["task"]
    
    task_prompts = {
        "review": "Review this code for bugs, security issues, and best practices:",
        "explain": "Explain what this code does in detail:",
        "optimize": "Suggest optimizations for this code:",
        "debug": "Help debug this code and identify potential issues:",
        "document": "Generate documentation for this code:"
    }
    
    prompt = f"""{task_prompts.get(task, 'Analyze this code:')}

Language: {language}

```{language}
{code}
```

Provide a detailed analysis."""
    
    return await gemini_chat({"message": prompt, "model": GEMINI_MODEL})

async def gemini_generate_code(args: dict) -> list[TextContent]:
    """Generate code with Gemini"""
    description = args["description"]
    language = args["language"]
    context = args.get("context", "")
    
    prompt = f"""Generate {language} code for the following:

{description}

{f'Context: {context}' if context else ''}

Provide clean, well-commented code following best practices."""
    
    return await gemini_chat({"message": prompt, "model": GEMINI_MODEL})

async def main():
    """Run the MCP server"""
    from mcp.server.stdio import stdio_server
    
    async with stdio_server() as (read_stream, write_stream):
        await app.run(
            read_stream,
            write_stream,
            app.create_initialization_options()
        )

if __name__ == "__main__":
    asyncio.run(main())
