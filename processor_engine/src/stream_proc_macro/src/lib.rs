use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(StreamBlockMacro)]
pub fn stream_processor_macro_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident; 
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut field_presence = vec![false; 7];
    let fields_names =match &ast.data {
        syn::Data::Struct(data_struct) => {
            for field in &data_struct.fields {
                if let Some(ident) = &field.ident {
                    match ident.to_string().as_str() {
                        "inputs" => field_presence[0] = true,
                        "outputs" => field_presence[1] = true,
                        "parameters" => field_presence[2] = true,
                        "statics" => field_presence[3] = true,
                        "module_name" => field_presence[4] = true,
                        "state" => field_presence[5] = true,
                        "lock" => field_presence[6] = true,
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
            "Struct must have 'inputs', 'outputs', 'parameters', 'statics', 'state', and 'lock' fields to derive MemoryVarTraid.",
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
                let m_state = self.state.lock().unwrap();
                *m_state == state
            }
            fn set_state(&mut self, state: StreamingState){
                *self.state.lock().unwrap() = state;
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
                Box::leak(format!("{}.{}", self.module_name, name).into_boxed_str())
            }
        }
        impl #impl_generics StreamBlock for #name #ty_generics #where_clause
        {
            fn new_input<V: 'static + Send>(&mut self, key: &'static str, description: &'static str) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                match self.inputs.insert(key, Box::new(Input::<V>::new(qualified_name, description))) {
                    Some(_) => Ok(()),
                    None => Err(StreamingError::InvalidOperation)
                }
            }
            fn new_output<V: 'static + Send + Clone> (&mut self, key: &'static str, description: &'static str) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                match self.outputs.insert(key, Box::new(Output::<V>::new(qualified_name, description))) {
                    Some(_) => Ok(()),
                    None => Err(StreamingError::InvalidOperation)
                }
            }
            fn new_parameter<V: 'static + Send + Sync + Copy + Clone + Serialize + PartialOrd> (&mut self, key: &'static str, description: &'static str, value: V) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                match self.parameters.insert(key, Box::new(Parameter::<V>::new(qualified_name, description, value))) {
                    Some(_) => Ok(()),
                    None => Err(StreamingError::InvalidOperation)
                }
            }
            fn new_statics<V: 'static + Send + Sync + Copy + Serialize> (&mut self, key: &'static str, description: &'static str, value: V) -> Result<(), StreamingError> {
                let qualified_name: &'static str = Self::get_qualified_name(self, key);
                match self.statics.insert(key, Box::new(Statics::<V>::new(qualified_name, value))) {
                    Some(_) => Ok(()),
                    None => Err(StreamingError::InvalidOperation)
                }
            }
            fn get_input<V: Send> (&self, key: &str) -> Result<&Input<V>, StreamingError> {
                if let Some(container) = self.inputs.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
                    if let Some(input_container) = any_ref.downcast_ref::<Input<V>>() {
                        Ok(input_container)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn get_output<V: Send> (&self, key: &str) -> Result<&Output<V>, StreamingError> {
                if let Some(container) = self.outputs.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
                    if let Some(output_container) = any_ref.downcast_ref::<Output<V>>() {
                        Ok(output_container)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidOutput)
                }
            }
            fn get_parameter<V : Send + Sync + 'static + Copy + Display> (&self, key: &str) -> Result<&Parameter<V>, StreamingError> {
                if let Some(container) = self.parameters.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
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
                if let Some(container) = self.statics.get(key) {
                    let any_ref: &dyn Any = container.as_any();
                    if let Some(statics) = any_ref.downcast_ref::<Statics<V>>() {
                        Ok(statics)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidStatics)
                }
            }

            fn get_input_channel<V: 'static + Send + Any>(&self, key: &str) -> Result<&Sender<V>, StreamingError> {
                if let Some(container) = self.inputs.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
                    if let Some(input_container) = any_ref.downcast_ref::<Input<V>>() {
                        Ok(&input_container.sender)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn connect<V: 'static + Send + Any + Clone>(&mut self, key: &str, sender: Sender<V>) -> Result<(), StreamingError> {
                if let Some(container) = self.outputs.get_mut(key) {
                    let any_mut: &mut dyn Any = container.as_mut();
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
            fn get_parameter_value<V:'static + Send + PartialOrd + Clone + Copy + Serialize + Sync>(&self, key: &str) -> Result<V, StreamingError> {
                if let Some(container) = self.parameters.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
                    if let Some(param) = any_ref.downcast_ref::<Parameter<V>>() {
                        Ok(param.get_value())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidParameter)
                }
            }
            fn set_parameter_value<V:'static + Send + PartialOrd + Clone + Copy + Serialize + Sync>(&mut self, key: &str, value: V) -> Result<(), StreamingError> {
                if let Some(container) = self.parameters.get_mut(key) {
                    let any_mut: &mut dyn Any = container.as_mut();
                    if let Some(param) = any_mut.downcast_mut::<Parameter<V>>() {
                        param.set_value(value)?;
                        Ok(())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidParameter)
                }
            }
            fn set_statics_value<V:'static + Send + Clone + Copy + Serialize + Sync>(&mut self, key: &str, value: V) -> Result<(), StreamingError> {
                if let Some(container) = self.statics.get_mut(key) {
                    let any_mut = container.as_mut().as_any_mut();
                    match any_mut {
                        None => Err(StreamingError::InvalidOperation),
                        Some(any_mut) => {
                            if let Some(statics) = any_mut.downcast_mut::<Statics<V>>() {
                                statics.set(value)
                            } else {
                                Err(StreamingError::WrongType)
                            }
                        }
                    }
                } else {
                    Err(StreamingError::InvalidStatics)
                }
            }
        }
    };
    eprintln!("Generated StreamBlock implementation for {}", name);
    eprintln!("{}", code_gen);
    code_gen.into()
}