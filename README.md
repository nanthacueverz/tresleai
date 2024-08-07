# Facade-Microservice 
## Overview
This Microservice will on-board the client application, the Facade Microservice might handle tasks such as user onboarding, initial setup, and integration with other services. It acts as a single point of contact for the client application, abstracting the complexity of the underlying microservices. 
## Getting Started
### Prerequisites
#### Installation
Rust 1.XX or higher
Kafka installation needed if testing from local.
Installation
If working from a local machine, clone the repository using git clone
#### Updating Sumbodule
1. Run the following command to update the submodule:
```
   $git submodule update --init --recursive
```
#### Activate SSH Tunnel
1. Activate SSH tunnel to connect to remote documentDB by updating the ${} values and running this command: 
```
   $ssh -i ${PATH_TO_YOUR_AUTHORIZED_SSH_PRIV_KEY} -L 127.0.0.1:27017:${DOCUMENT_DB_CLUSTER_HOST}:27017 ${BASTION_USER}@${BASTION_HOST} -N
```
Note: An '&' can be added at the end of this command to keep the tunnel open while you still work on the same terminal window

## Usage of commands
### Running code
1. Incorporate the correct mongo_db_url into the global.yaml file in the configurations folder:
```
mongodb://<UPDATE_USERNAME>:<UPDATE_PASSWORD>@localhost:27017/?tls=true&tlsCAFile=src%2Ftest%2Fglobal-bundle.pem&directConnection=true&tlsAllowInvalidCertificates=true&retryWrites=false
Note: This URL is not unique to each connection
```

2. Once cloned, run this microservice using the following command from the terminal:
```
    $cargo run
```
This will start the service.

### Microservice validation with Facade APIs

1. Once the message 'Server started successfully' appears on the terminal, you can access the Facade APIs using the automatically generated [Swagger UI](http://localhost:8000/swagger-ui/)
2. Navigate to the /api/v1.1/admin/apps dropdown and select Execute. Then verify that the number of apps fetched successfully matches the app count for verification (findable in the response body).

### Steps to be done if code is updated
1.cargo clippy is a command that checks your Rust code for common mistakes and style issues. It's a linter that provides helpful suggestions to improve your code.
To use cargo clippy, you first need to install it. You can do this by running the following command in your terminal:
```
    $rustup component add clippy
```
Once clippy is installed, you can run it on your project by navigating to your project directory in the terminal and running:
```
    $cargo clippy
```
It's a good practice to run cargo clippy regularly and especially before committing your code to ensure it's as clean and efficient as possible.
2.For checking the rust code formatting issues, we use `cargo fmt --check`.
To use cargo fmt --check, you first need to install rustfmt. You can do this by running the following command in your terminal:
```
   $rustup component add clippy
```
Once rustfmt is installed, you can check your project's formatting by navigating to your project directory in the terminal and running:

```
    $cargo fmt --check
```
This command will check all the files in your project and report any formatting issues without actually modifying the code. If you want to automatically fix the formatting issues, you can run the below command.
```
    $cargo fmt
```
This command will automatically format all the files in your project according to the official Rust formatting guidelines. It's a good practice to run cargo fmt regularly and especially before committing your code to ensure it adheres to the official Rust formatting guidelines.

## Build and run locally using docker/docker compose

[docker-compose.yml](./docker-compose.yml) file spins ups a local kafka, akhq and tresleai facade service.
We are using [dev.Dockerfile](./dev.Dockerfile) for local dev (for enhanced caching using docker layers).

1. Pre-requisites (only one-time setup):
   
   Note: For yaml files, get the values for <UPDATE> from a teammate if you don't have it :)

   1. Create your own global.yaml file 
      
      ```
      $ cp configurations/global.yaml configurations/.global.yaml
      ```

   2. Update the username:password in the mongo_db_url config in the new [.global.yaml](./configurations/.global.yaml) file

   3. Create your own local.yaml file
   
      ```
      $ cp configurations/local.yaml configurations/.local.yaml
      ```
   4. Setup an ssh tunnel to connect to remote documentDB(if not already completed earlier)

      Update the ${} values before running the command. Note there is an '&' at the end to keep the tunnel open while you still work on the same terminal window:
   
      ```
      $ ssh -i ${PATH_TO_YOUR_AUTHORIZED_SSH_PRIV_KEY} -L 127.0.0.1:27017:${DOCUMENT_DB_CLUSTER_HOST}:27017 ${BASTION_USER}@${BASTION_HOST} -N &
      ```
      
      Currently, we are using a shared documentDB for testing.
      This is because we have a number of microservices that create different collections that are needed at runtime.
   
   5. Create docker network for service to easily communicate with each other
      
      ```
      $ docker network create tresleai
      Note: Create new network if tresleai network not already created
      ```
      
   6. Create aws credential custom profile
      
      This is needed to connect to s3 and other aws services, depending on the access the aws key has.
      ```
      $ aws configure --profile dev
      # follow the prompts to set the correct key, id and region
      ```
      
      Notes:
      1. The aws local directory ~/.aws/ is mounted as read only volume in the docker container.
      2. For staging, create a new profile and update the AWS_PROFILE env in the docker-compose.yml file.

2. Build and run

    ```
    docker compose up --build -d
    
    # check status of 3 services (kafka, akhq, tresleai-facade-service) - should be Up or Healthy
    docker compose ps
    ```

3. (For continuous dev iteration) Rebuild only tresleai-facade-service

   For changes in source code: rust or python files:
   ```
   docker compose up --build -d
   ```

   For changes made only in `configurations` folder or `envs` in docker-compose file:
   ```
   docker compose up --force-recreate tresleai-facade-service -d
   ```

4. Check logs

    ```
    # tail logs from only one service (get service name from docker-compose.yml > services)
    docker compose log -f tresleai-facade-service
   
    # tail logs of all services in docker-compose.yml
    docker compose logs -f
    ```

5. Access and manage local kafka

   You can access your local kafka using at: http://localhost:8080/
   
   On successful start, you should see `apponboard` and `appdelete` topics already created using 
   the akhq dashboard [here](http://localhost:8080/ui/docker-kafka-server/topic)

   You can now send messages to these topics from the dashboard.

6. Simulate/produce data source update events in kafka

   Checkout out this [kafka](./docs/kafka.md) doc for more info

7. Stop the services

   ```
   docker compose down
   ```

8. Nuke everything (optional)

   * Delete the docker containers and volume mounts
   * Delete the cached volumes
   * Close the ssh tunnel
   
      ```
      make nuke
      ```
     
   Note: docker network is not nuked because we might be using the same named network for other dockerized microservices.

### Troubleshooting

Facing issues? Checkout the [troubleshooting guide](./docs/troubleshooting-guide.md)

## Features
The Facade Microservice provides the following features:
### admin_ui_api
#### app_delete_handler -
    This api deletes an app from the DocumentDB and other associated resources.
    as shown below :
    ```
        /api/v1.1/admin/apps/{app_name}
    ```
#### app_get_handler -
    This api is a GET handler for fetching an app from DocumentDB.
    ```
        /api/v1.1/admin/apps/{app_name}
    ```
#### app_get_logs_handler -
    This api is a GET handler to fetch the logging data for the app.
    ```
        /api/v1.1/admin/logs
    ```
#### app_knowledge_nodes_and_errors_count -
    This api is a GET handler to fetch count of knowledge nodes and errors while processing them for an app between two timestamps.
    ```
        /api/v1.1/admin/nodes/count/{app_name}
    ```
#### app_knowledge_nodes_chart_handler -
    This api is a GET handler that fetches the data from the knowledge nodes for an app between two timestamps. The data is then displayed on a chart on admin UI.
    ```
        /api/v1.1/admin/nodes/chart/{app_name}
    ```
#### app_knowledge_node_error_handler -
    This api is GET handler to fetch errors while processing/extracting knowledge nodes for an app between two timestamps.
    ```
        /api/v1.1/admin/nodes/errors/{app_name}
    ```
#### app_knowledge_nodes_handler -
    This api is GET handler to fetch knowledge nodes for an app between two timestamps.
    ```
        GET handler to fetch knowledge nodes for an app between two timestamps.
    ```
#### app_list_handler - 
    This api is GET handler for fetching the list of onboarded apps from DocumentDB.
    ```
        /api/v1.1/admin/apps
    ```
#### app_search_enabled_handler -
    This api(patch) updates the search_enabled flag of an app in DocumentDB.
    ```
        /api/v1.1/admin/search/apps/{app_name}
    ```
#### apps_and_calls_overview_handler -
    This api is a GET handler to fetch the overview of calls made from different apps during the last 6 months.
    ```
        /api/v1.1/admin/overview
    ```
#### kub_generate_token_handler -
    This api is a GET handler that generates a token to login into kubernetes dashboard.
    ```
        /api/v1.1/admin/token
    ```
#### metric_calls_handler -
    This api is a GET handler that fetches the number of metric calls made to the app.
    ```
        /api/v1.1/admin/metric/calls
    ```
#### metric_error_handler -
    This api is a GET handler that fetches the number of errors made to the app.
    ```
        /api/v1.1/admin/metric/logs
    ```
### onboarding
#### handler - 
    This module contains the POST handler for onboarding/updating an app and calls helper functions to
    perform operations with DocumentDB and Kafka.
    ```
        /api/v1.1/admin/apps/onboard
    ```
### retrieval - 
#### handler -
    This is POST handler to initiate the retrieval process for a response corresponding to a specific query.This API triggers a retrieval operation in the backend, which fetches data based on the user details and query provided in the request. The retrieval process involves validating user policies, and asynchronously initiating the response retrieval by passing on the query and user details to the engine.
    ```
        /api/v1.0/retrieval
    ```
    GET handler to extract a specific document from an application's history collection, with an input 'reference_id' as the basis for retrieval.
    ```
        /api/v1.0/history/retrieval
    ```
### Integrates with pheripheral services -
    1. This service records informational or error logs in the Logging Microservice.
    2. It logs metric data in the Metric Microservice.
    3. It also logs audit data and error in the Audit Microservice.
