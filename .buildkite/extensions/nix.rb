class NixExt < Buildkite::Builder::Extension
  dsl do
    # Runs `cmd` inside the named devShell from this repo's flake.nix.
    def nix(shell, dir, cmd, emoji: :nix, label: nil, env_vars: nil, agents: { "queue": "default" }, retry_attempts: 2, timeout_mins: nil)
      command do
        label label.nil? ? cmd.to_s : label, emoji: emoji
        env env_vars unless env_vars.nil?
        agents agents unless agents.nil?
        timeout_in_minutes timeout_mins unless timeout_mins.nil?
        automatic_retry_on(exit_status: -1, limit: retry_attempts)
        command "cd #{dir}"
        command "nix develop $$BUILDKITE_BUILD_CHECKOUT_PATH##{shell} -c bash -c \"#{cmd}\""
      end
    end
  end
end
