import { Server } from "@modelcontextprotocol/sdk/server/index.js";
const server = new Server({ name: "my-mcp", version: "0.1.0" }, { capabilities: {} });
export { server };
