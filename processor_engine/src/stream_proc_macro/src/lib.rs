use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(StreamBlockMacro)]
pub fn stream_processor_macro_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident; 
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut field_presence = vec![false; 8];
    let fields_names =match &ast.data {
        syn::Data::Struct(data_struct) => {
            for field in &data_struct.fields {
                if let Some(ident) = &field.ident {
                    match ident.to_string().as_str() {
                        "inputs" => field_presence[0] = true,
                        "outputs" => field_presence[1] = true,
                        "parameters" => field_presence[2] = true,
                        "statics" => field_presence[3] = true,
                        "state" => field_presence[4] = true,
                        "name" => field_presence[5] = true,
                        "proc_state" => field_presence[6] = true,
                        "lock" => field_presence[7] = true,
                        _ => {}
                    }
                }
            }
            field_presence.iter().all(|&x| x) // Raccoglie i risultati di ogni 'Some(*acc)' in un nuovo Vec<bool>
        }
        _ => false,
    };

    if !fields_names {
        return syn::Error::new_spanned(
            name,
            "Struct must have 'inputs', 'outputs', 'parameters', 'statics', 'state', 'name', 'prooc_state' and 'lock' fields to derive StreamBlockMacro.",
        )
        .to_compile_error()
        .into();
    }

    let code_gen = quote! {
        impl #impl_generics StreamBlockDyn for #name #ty_generics #where_clause {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
            fn check_state(&self, state: StreamingState) -> bool {
                let m_state = self.proc_state.lock().unwrap();
                *m_state == state
            }
            fn set_state(&mut self, state: StreamingState){
                *self.proc_state.lock().unwrap() = state;
            }
            fn get_input_list(&self) -> Vec<&str>{
                self.inputs.keys().copied().collect()
            }
            fn get_output_list(&self) -> Vec<&str>{
                self.outputs.keys().copied().collect()
            }
            fn get_parameter_list(&self) -> Vec<&str>{
                self.parameters.keys().copied().collect()
            }
            fn get_statics_list(&self) -> Vec<&str>{
                self.statics.keys().copied().collect()
            }
            fn is_initialized(&self) -> bool {
                let keys = self.get_statics_list();
                for k in keys {
                    let statics_dyn = self.statics.get(k).unwrap();
                    if statics_dyn.is_settable() {
                        return false;
                    }
                }
                return true;
            }
            fn get_qualified_name(&self, name: &str) -> &'static str {
                Box::leak(format!("{}.{}", self.name, name).into_boxed_str())
            }
        }
        impl #impl_generics StreamBlock for #name #ty_generics #where_clause
        {
            fn new_input<V: 'static + Send + Clone>(&mut self, key: &'static str) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if self.inputs.contains_key(qualified_name) {
                    return Err(StreamingError::AlreadyDefined);
                }
                self.inputs.insert(qualified_name, Box::new(Input::<V>::new(qualified_name)));
                Ok(())
            }
            fn new_output<V: 'static + Send + Clone> (&mut self, key: &'static str) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if self.outputs.contains_key(qualified_name) {
                    return Err(StreamingError::AlreadyDefined);
                }
                self.outputs.insert(qualified_name, Box::new(Output::<V>::new(qualified_name)));
                Ok(())
            }
            fn new_state<V: 'static + Send + Sync + Clone + Serialize + PartialOrd + Display> (&mut self, key: &'static str, value: V,) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if self.state.contains_key(qualified_name) {
                    return Err(StreamingError::AlreadyDefined);
                }
                self.state.insert(qualified_name, Box::new(State::<V>::new(qualified_name, value)));
                Ok(())
            }
            fn new_parameter<V: 'static + Send + Sync + Clone + Serialize + PartialOrd + Display> (&mut self, key: &'static str, value: V, limits: Option<[V;2]>) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if self.parameters.contains_key(qualified_name) {
                    return Err(StreamingError::AlreadyDefined);
                }
                self.parameters.insert(qualified_name, Box::new(Parameter::<V>::new(qualified_name, value, limits)));
                Ok(())
            }
            fn new_statics<V: 'static + Send + Sync + Clone + Serialize + PartialOrd + PartialEq+Display> (&mut self, key: &'static str, value: V, limits: Option<[V;2]>) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if self.statics.contains_key(qualified_name) {
                    return Err(StreamingError::AlreadyDefined);
                }
                self.statics.insert(qualified_name, Box::new(Statics::<V>::new(qualified_name, value, limits)));
                Ok(())
            }
            fn get_input<V: Send+Clone> (&self, key: &str) -> Result<&Input<V>, StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.inputs.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(input_container) = any_ref.downcast_ref::<Input<V>>() {
                        Ok(input_container)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn get_output<V: Send+Clone> (&self, key: &str) -> Result<&Output<V>, StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.outputs.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(output_container) = any_ref.downcast_ref::<Output<V>>() {
                        Ok(output_container)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidOutput)
                }
            }
            fn get_parameter<V : 'static + Send + Sync + Clone + Display> (&self, key: &str) -> Result<&Parameter<V>, StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.parameters.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(param) = any_ref.downcast_ref::<Parameter<V>>() {
                        Ok(param)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidParameter)
                }
            }
            fn get_statics<V: 'static + Send + Sync + Display> (&self, key: &str) -> Result<&Statics<V>, StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.statics.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(statics) = any_ref.downcast_ref::<Statics<V>>() {
                        Ok(statics)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidStatics)
                }
            }

            fn get_input_channel<V: 'static + Send + Any + Clone>(&self, key: &str) -> Result<SyncSender<V>, StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.inputs.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(input_container) = any_ref.downcast_ref::<Input<V>>() {
                        Ok(input_container.sender.clone())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn connect<V: 'static + Send + Any + Clone>(&mut self, key: &str, sender: SyncSender<V>) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.outputs.get_mut(qualified_name) {
                    let any_mut: &mut dyn Any = container.as_mut().as_any_mut();
                    if let Some(output) = any_mut.downcast_mut::<Output<V>>() {
                        output.connect(sender);
                        Ok(())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidOutput)
                }
            }
            fn get_parameter_value<V:'static + Send + PartialOrd + Clone + Serialize + Sync + Display>(&self, key: &str) -> Result<V, StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.parameters.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(param) = any_ref.downcast_ref::<Parameter<V>>() {
                        Ok(param.get_value())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidParameter)
                }
            }
            fn set_parameter_value<V:'static + Send + PartialOrd + Clone + Serialize + Sync + Display>(&mut self, key: &str, value: V) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.parameters.get_mut(qualified_name) {
                    let any_mut: &mut dyn Any = container.as_mut().as_any_mut();
                    if let Some(param) = any_mut.downcast_mut::<Parameter<V>>() {
                        param.set_value(value)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidParameter)
                }
            }
            fn set_statics_value<V:'static + Send + Clone + Serialize + Sync + PartialOrd + PartialEq+Display>(&mut self, key: &str, value: V) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.statics.get_mut(qualified_name) {
                    let any_mut: &mut dyn Any = container.as_mut().as_any_mut();
                    if let Some(statics) = any_mut.downcast_mut::<Statics<V>>() {
                        statics.set_value(value)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidParameter)
                }
            }
            fn recv_input<V: 'static + Send+Clone> (&mut self, key: &str) -> Result<V , StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.inputs.get_mut(qualified_name) {
                    let any_ref : &mut dyn Any = container.as_mut().as_any_mut(); 
                    if let Some(input_container) = any_ref.downcast_mut :: < Input < V >> () { 
                        input_container.recv() 
                    } else { 
                        Err(StreamingError :: WrongType) 
                    }
                } else { 
                    Err(StreamingError :: InvalidInput) 
                }
            }
            fn send_output<V:'static + Send+Clone> (&self, key: &str, value: V) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                if let Some(container) = self.outputs.get(qualified_name) {
                    let any_ref: &dyn Any = container.as_ref().as_any();
                    if let Some(output_container) = any_ref.downcast_ref::<Output<V>>() {
                        output_container.send(value)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidOutput)
                }
            }
        }
    };
    //eprintln!("Generated StreamBlock implementation for {}", name);
    //eprintln!("{}", code_gen);
    code_gen.into()
}