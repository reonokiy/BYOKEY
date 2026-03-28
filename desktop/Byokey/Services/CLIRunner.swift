import Foundation

enum CLIRunner {
    static var binaryURL: URL { ProcessManager.binaryURL }

    static func login(provider: String, account: String? = nil) async throws {
        let process = Process()
        process.executableURL = binaryURL
        var arguments = ["login", provider]
        if let account {
            arguments += ["--account", account]
        }
        process.arguments = arguments

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        try process.run()
        process.waitUntilExit()

        if process.terminationStatus != 0 {
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw CLIError.loginFailed(output)
        }
    }

    enum CLIError: LocalizedError {
        case loginFailed(String)

        var errorDescription: String? {
            switch self {
            case .loginFailed(let output):
                "Login failed: \(output)"
            }
        }
    }
}
