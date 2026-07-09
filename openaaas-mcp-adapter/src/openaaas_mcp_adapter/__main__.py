import sys
from fastmcp import FastMCP
from .tools import register_tools


def main() -> None:
    mcp = FastMCP("openaaas-mcp-adapter")
    register_tools(mcp)
    mcp.run(transport="stdio")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
