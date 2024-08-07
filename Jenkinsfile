pipeline {
    agent any

    stages {
        stage('Test') {
            steps {
                // Run the unit tests
                cargo test
                echo 'Unit tests passed'
                // connect to document db
            }
        }
    }
}