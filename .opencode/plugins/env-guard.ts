export default function envGuard() {
  return {
    on: {
      "tool.execute.before"(event: any) {
        const input = JSON.stringify(event.properties || {})
        if (input.includes(".env")) {
          throw new Error("Access to .env files is blocked by project policy.")
        }
      }
    }
  }
}
