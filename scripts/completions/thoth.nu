module completions {

  def "nu-complete thoth view" [] {
    ^thoth list
    | lines 
    | parse "{value}"
    | each { |item| 
        if ($item.value | str contains " ") {
          $'"($item.value)"'
        } else {
          $item.value
        }
      }
  }

  def "nu-complete thoth delete" [] {
    ^thoth list
    | lines 
    | parse "{value}"
    | each { |item| 
        if ($item.value | str contains " ") {
          $'"($item.value)"'
        } else {
          $item.value
        }
      }
  }

  def "nu-complete thoth copy" [] {
    ^thoth list
    | lines 
    | parse "{value}"
    | each { |item| 
        if ($item.value | str contains " ") {
          $'"($item.value)"'
        } else {
          $item.value
        }
      }
  }
  export extern "thoth view" [
     name: string@"nu-complete thoth view"
  ]
  export extern "thoth delete" [
     name: string@"nu-complete thoth delete"
  ]
  export extern "thoth copy" [
     name: string@"nu-complete thoth copy"
  ]
}


export use completions *
