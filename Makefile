.PHONY: cleanup

nuke:
	@echo "Shutting down the containers..."
	@docker compose down || true
	@echo "Removing volume mounts..."
	@docker compose rm -vf || true
	@echo "Deleting the cached volumes..."
	@if docker volume ls -qf "name=tresleai-facade-service" | grep -q . ; then docker volume rm $$(docker volume ls -qf "name=tresleai-facade-service"); fi
	@echo "Closing the ssh tunnel..."
	@pgrep -f "127.0.0.1:27017" | xargs -r kill -9 || true
	@echo "Cleaned up successfully"