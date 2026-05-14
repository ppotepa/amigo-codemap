fn main() -> anyhow::Result<()> {
    amigo_codemap::daemon::run_from_env_args()
}
