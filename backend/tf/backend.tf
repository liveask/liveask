terraform {
  cloud {
    organization = "liveask"

    workspaces {
      name = "liveask-prod-eu-west"
    }
  }
}
