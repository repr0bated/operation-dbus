#!/bin/bash

# Setup script for MCP fork sync
# This will configure the sync to work with your existing fork

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}MCP Fork Sync Setup${NC}"
echo "===================="

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
fi

# Get current repository info
CURRENT_REPO=$(git remote get-url origin)
echo -e "${YELLOW}Current repository: ${CURRENT_REPO}${NC}"

# Ask for fork repository URL
echo ""
echo "Please provide your MCP fork repository details:"
read -p "Fork repository URL (e.g., https://github.com/user/mcp-dbus): " FORK_URL

if [ -z "$FORK_URL" ]; then
    echo -e "${RED}Error: No fork URL provided${NC}"
    exit 1
fi

# Extract repository name from URL
FORK_REPO=$(echo "$FORK_URL" | sed 's/.*github\.com\///' | sed 's/\.git$//')
echo -e "${YELLOW}Fork repository: ${FORK_REPO}${NC}"

# Test connection to fork
echo -e "${YELLOW}Testing connection to fork...${NC}"
if git ls-remote "$FORK_URL" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Fork is accessible${NC}"
else
    echo -e "${RED}✗ Cannot access fork. Please check the URL and permissions${NC}"
    exit 1
fi

# Create .env file for local sync
echo -e "${YELLOW}Creating local configuration...${NC}"
cat > .mcp-sync.env << EOF
# MCP Fork Sync Configuration
MCP_FORK_REPO=$FORK_REPO
MCP_FORK_URL=$FORK_URL
EOF

echo -e "${GREEN}✓ Local configuration created${NC}"

# Update the sync script to use the fork
echo -e "${YELLOW}Updating sync script...${NC}"
sed -i "s|FORK_REPO=\"\${1:-\${MCP_FORK_REPO}}\"|FORK_REPO=\"\${1:-\${MCP_FORK_REPO:-$FORK_URL}}\"|" sync-to-mcp-fork.sh

echo -e "${GREEN}✓ Sync script updated${NC}"

# Test the sync (dry run)
echo -e "${YELLOW}Testing sync (dry run)...${NC}"
if ./sync-to-mcp-fork.sh --dry-run 2>/dev/null; then
    echo -e "${GREEN}✓ Sync test passed${NC}"
else
    echo -e "${YELLOW}⚠ Sync test had issues, but configuration is saved${NC}"
fi

# Show next steps
echo ""
echo -e "${GREEN}Setup Complete!${NC}"
echo "==============="
echo ""
echo "Next steps:"
echo "1. Run sync: ./sync-to-mcp-fork.sh"
echo "2. For GitHub Actions, add this secret to your repository:"
echo "   MCP_FORK_REPO = $FORK_REPO"
echo ""
echo "To sync now, run:"
echo -e "${YELLOW}  ./sync-to-mcp-fork.sh${NC}"
echo ""
echo "To set up automatic sync via GitHub Actions:"
echo "1. Go to: https://github.com/$(git remote get-url origin | sed 's/.*github\.com\///' | sed 's/\.git$//')/settings/secrets/actions"
echo "2. Add secret: MCP_FORK_REPO = $FORK_REPO"
echo "3. Push changes to trigger automatic sync"