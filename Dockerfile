# Use Ubuntu as the base image
FROM rust:latest

# Set the working directory
WORKDIR /usr/src/app

# Install necessary dependencies
RUN apt-get update && \
    apt-get install -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the executable from the local machine to the container
COPY target/release/tresleai-uifacade-service .

# Set executable permissions during build
RUN chmod +x ./tresleai-uifacade-service
RUN wget https://truststore.pki.rds.amazonaws.com/global/global-bundle.pem -O /usr/src/app/global-bundle.pem

# Execute the default command specified in the CMD instruction
CMD ["./tresleai-uifacade-service"]
