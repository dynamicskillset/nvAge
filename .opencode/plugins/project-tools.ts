import { tool } from "opencode/plugin"
import { z } from "zod"

export default function projectTools({ $, client }: any) {
  return {
    tools: {
      run_project_checks: tool({
        description: "Run the standard lint and test commands for this project.",
        args: z.object({
          mode: z.enum(["quick", "full"]).default("quick")
        }),
        async execute(args: { mode: "quick" | "full" }) {
          const command =
            args.mode === "quick"
              ? "npm run lint && npm test"
              : "npm run lint && npm test && npm run build"

          const result = await $`${["bash", "-lc", command]}`
          client.app.log("info", "Ran project checks", { mode: args.mode })
          return {
            title: "Project checks completed",
            output: String(result.stdout || "")
          }
        }
      })
    }
  }
}
